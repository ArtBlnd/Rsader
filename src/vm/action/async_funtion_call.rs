use crate::{
    ir::ty::IrType,
    vm::{context::VmContext, function::Function, value::Value, VmError},
};

use super::{Action, ActionBehavior, ActionState, TaskContext};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum AsyncFunctionCallState {
    #[default]
    Evaluating,
}

impl ActionState for AsyncFunctionCallState {
    fn state(&self) -> &'static str {
        match self {
            AsyncFunctionCallState::Evaluating => "RUNNING",
        }
    }
}

#[derive(thiserror::Error, Debug, Clone, PartialEq, Eq)]
pub enum AsyncFunctionCallError {
    #[error("Failed to evaluate input: {0}")]
    EvaluateError(#[from] VmError),
}

pub struct AsyncFunctionCallAction {
    pub function: Function,
    pub args0: Option<Vec<IrType>>,
    pub args1: Option<Vec<Value>>,

    pub context: VmContext,
}

impl Action for AsyncFunctionCallAction {
    const NAME: &'static str = "async_fn_call";

    type ExecutionResult = Value;
    type State = AsyncFunctionCallState;
    type Error = AsyncFunctionCallError;

    async fn execute(
        &mut self,
        context: &TaskContext,
        _state: &mut Self::State,
    ) -> Result<ActionBehavior<Self::ExecutionResult>, Self::Error> {
        match self
            .function
            .call(
                context,
                self.args0.take().unwrap(),
                self.args1.take().unwrap(),
                &mut self.context,
            )
            .await
        {
            Ok(value) => Ok(ActionBehavior::Stop(value)),
            Err(e) => Err(AsyncFunctionCallError::EvaluateError(e)),
        }
    }

    async fn execute_fallback(
        &mut self,
        _context: &TaskContext,
        _state: Self::State,
        _error: Option<Self::Error>,
    ) -> Result<(), Self::Error> {
        Ok(())
    }
}
