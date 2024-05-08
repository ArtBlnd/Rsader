use std::convert::Infallible;
use std::error::Error as StdError;
use std::panic::AssertUnwindSafe;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;

use futures::Future;
use futures::FutureExt;

mod async_funtion_call;
pub use async_funtion_call::*;
mod evaluate;
pub use evaluate::*;
mod order;
pub use order::*;
mod tick_trade;
pub use tick_trade::*;
mod mm_tick_trade;
pub use mm_tick_trade::*;

use crate::global_context::AsyncHandle;
use crate::global_context::GlobalContext;
use crate::ir::ty::IrType;
use crate::ir::ty::TypeOf;
use crate::utils::async_helpers;
use crate::utils::maybe_trait::MaybeSend;
use crate::window::console::ConsoleInstrument;
use crate::window::console::ConsolePipe;

use super::value::IntoValue;
use super::value::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionBehavior<R> {
    Stop(R),
    Wait(Duration),
    Continue,
}

pub trait ActionState: Default {
    fn state(&self) -> &'static str;
}

impl ActionState for () {
    fn state(&self) -> &'static str {
        "RUNNING"
    }
}

#[cfg_attr(not(target_arch = "wasm32"), trait_variant::make(Send))]
pub trait Action {
    const NAME: &'static str;

    type ExecutionResult: IntoValue + MaybeSend;
    type State: ActionState + MaybeSend;
    type Error: StdError + MaybeSend;

    async fn execute(
        &mut self,
        context: &TaskContext,
        state: &mut Self::State,
    ) -> Result<ActionBehavior<Self::ExecutionResult>, Self::Error>;

    async fn execute_fallback(
        &mut self,
        context: &TaskContext,
        state: Self::State,
        error: Option<Self::Error>,
    ) -> Result<(), Self::Error>;
}

pub struct ActionFuture<F> {
    inner: Option<F>,
    ty: IrType,
}

impl<F> ActionFuture<F> {
    pub fn new(inner: F, ty: IrType) -> Self {
        Self {
            inner: Some(inner),
            ty,
        }
    }
}

impl<F> TypeOf for ActionFuture<F> {
    fn type_of(&self) -> IrType {
        self.ty.clone()
    }
}

impl<F> Action for ActionFuture<F>
where
    F: Future + MaybeSend,
    F::Output: IntoValue + MaybeSend,
{
    const NAME: &'static str = "custom";

    type ExecutionResult = F::Output;
    type State = ();
    type Error = Infallible;

    async fn execute(
        &mut self,
        _: &TaskContext,
        _: &mut Self::State,
    ) -> Result<ActionBehavior<Self::ExecutionResult>, Self::Error> {
        let inner = self.inner.take().unwrap();
        Ok(ActionBehavior::Stop(inner.await))
    }

    async fn execute_fallback(
        &mut self,
        _: &TaskContext,
        _: Self::State,
        _: Option<Self::Error>,
    ) -> Result<(), Self::Error> {
        Ok(())
    }
}

pub struct ActionToken {
    context: TaskContext,

    return_type: IrType,
    handle: AsyncHandle<Option<Value>>,
}

impl ActionToken {
    pub fn ty(&self) -> &IrType {
        &self.return_type
    }

    pub fn uuid(&self) -> uuid::Uuid {
        self.context.uuid
    }

    pub fn state(&self) -> &'static str {
        *self.context.state_str.lock().unwrap()
    }

    pub async fn join(self) -> Option<Value> {
        self.handle.await_handle().await
    }

    pub fn is_stopped(&self) -> bool {
        self.handle.is_finished()
    }

    pub fn cancel(&self) {
        self.context
            .stop_flag
            .store(true, std::sync::atomic::Ordering::SeqCst);
    }

    pub fn cancle_force(&self) {
        self.handle.abort();
        self.context
            .stop_flag
            .store(true, std::sync::atomic::Ordering::SeqCst);
    }
}

pub struct ActionExecutor<A>
where
    A: Action,
{
    action: A,
    ty: IrType,
}

#[derive(Clone)]
pub struct TaskContext {
    state_str: Arc<Mutex<&'static str>>,
    uuid: uuid::Uuid,
    stop_flag: Arc<AtomicBool>,
}

impl<A> ActionExecutor<A>
where
    A: Action + TypeOf + 'static,
{
    pub fn new(action: A) -> Self {
        let ty = action.type_of();
        Self { action, ty }
    }
}

impl<A> ActionExecutor<A>
where
    A: Action + 'static,
{
    pub fn new_with_type(action: A, ty: IrType) -> Self {
        Self { action, ty }
    }

    pub fn spawn_rt(self, ctx: &GlobalContext, console_pipe: ConsolePipe) -> ActionToken {
        let context = TaskContext {
            state_str: Arc::new(Mutex::new(A::State::default().state())),
            uuid: uuid::Uuid::new_v4(),
            stop_flag: Arc::new(AtomicBool::new(false)),
        };

        let action = self.action;
        let state = A::State::default();
        ActionToken {
            context: context.clone(),
            return_type: self.ty,
            handle: ctx.spawn(ConsoleInstrument::new(
                execute_internal(action, state, context),
                console_pipe,
            )),
        }
    }

    pub async fn execute(self, ctx: TaskContext) -> Option<Value> {
        let action = self.action;
        let state = A::State::default();
        let result = execute_internal(action, state, ctx).await;

        result.map(|v| v.into())
    }
}

async fn execute_internal<A>(
    mut action: A,
    mut action_state: A::State,
    context: TaskContext,
) -> Option<Value>
where
    A: Action + 'static,
{
    let state_str = context.state_str.lock().unwrap().clone();
    loop {
        *context.state_str.lock().unwrap() = action_state.state();
        if context.stop_flag.load(std::sync::atomic::Ordering::SeqCst) {
            tracing::info!("{}: action is cancelled", context.uuid);
            if let Err(e) = action.execute_fallback(&context, action_state, None).await {
                tracing::error!("{}: fallback failed: {}", A::NAME, e);
            };

            return None;
        }

        let e = match AssertUnwindSafe(action.execute(&context, &mut action_state))
            .catch_unwind()
            .await
        {
            Ok(Ok(ActionBehavior::Stop(r))) => {
                tracing::info!("{}: action is finished", context.uuid);
                *context.state_str.lock().unwrap() = state_str;
                return Some(r.into_value());
            }
            Ok(Ok(ActionBehavior::Continue)) => continue,
            Ok(Ok(ActionBehavior::Wait(dur))) => {
                async_helpers::sleep(dur).await;
                continue;
            }
            Ok(Err(e)) => e,
            Err(_) => {
                tracing::error!("{}: action panicked", context.uuid);

                context
                    .stop_flag
                    .store(true, std::sync::atomic::Ordering::SeqCst);
                return None;
            }
        };

        tracing::error!("{}: action failed: {}", context.uuid, e);
        if let Err(e) = action
            .execute_fallback(&context, action_state, Some(e))
            .await
        {
            tracing::error!("{}: fallback failed: {}", context.uuid, e);
        };

        context
            .stop_flag
            .store(true, std::sync::atomic::Ordering::SeqCst);
        return None;
    }
}
