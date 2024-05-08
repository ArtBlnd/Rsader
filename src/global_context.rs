use std::{
    pin::Pin,
    sync::{atomic::AtomicBool, Arc},
    task::{Context, Poll},
};

use crate::{
    broadcast::{self, Broadcaster},
    exchange::Exchanges,
    vm::{action::ActionToken, builtin_functions, builtin_ident, context::VmContext},
};

use async_channel::{unbounded, Sender as AsyncSender};
use futures::{Future, Stream};
use parking_lot::Mutex;

#[cfg(not(target_arch = "wasm32"))]
use tokio::runtime::Runtime;

#[derive(Clone)]
pub struct GlobalContext {
    ex: Exchanges,
    #[cfg(not(target_arch = "wasm32"))]
    rt: Arc<Runtime>,
    actions: Arc<Mutex<Vec<ActionToken>>>,

    broadcaster: Broadcaster,
    refresh_now: AsyncSender<()>,
}

impl GlobalContext {
    pub fn new(
        ex: Exchanges,
        broadcaster: broadcast::Broadcaster,
    ) -> (Self, impl Stream<Item = ()>) {
        let (tx, rx) = unbounded();

        (
            Self {
                ex,
                #[cfg(not(target_arch = "wasm32"))]
                rt: Arc::new(Runtime::new().unwrap()),
                actions: Arc::new(Mutex::new(Vec::new())),

                broadcaster,
                refresh_now: tx,
            },
            futures::stream::unfold(rx, |rx| async move { Some((rx.recv().await.unwrap(), rx)) }),
        )
    }

    pub fn ex(&self) -> &Exchanges {
        &self.ex
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn spawn<F>(&self, f: F) -> AsyncHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send,
    {
        AsyncHandle {
            handle: self.rt.spawn(f),
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub fn spawn<F>(&self, f: F) -> AsyncHandle<F::Output>
    where
        F: Future + 'static,
        F::Output: Send,
    {
        let (tx, rx) = async_channel::bounded(1);
        let cancellation = Arc::new(AtomicBool::new(false));

        wasm_bindgen_futures::spawn_local(CancelableFuture {
            inner: async move {
                let _ = tx.send(f.await).await;
            },
            cancel: cancellation.clone(),
        });

        AsyncHandle {
            cancellation,
            waiter: rx,
        }
    }

    pub fn broadcaster(&self) -> Broadcaster {
        self.broadcaster.clone()
    }

    pub fn new_vm_context(&self) -> VmContext {
        let mut ctx = VmContext::new(self.clone());
        builtin_functions::register_builtin_functions(&mut ctx);
        builtin_ident::register_builtin_identifiers(&mut ctx);
        ctx
    }

    pub fn insert_action(&self, action: ActionToken) {
        self.actions.lock().push(action);
    }

    pub fn remove_action(&self, uuid: uuid::Uuid) -> ActionToken {
        let mut actions = self.actions.lock();
        let index = actions.iter().position(|a| a.uuid() == uuid).unwrap();
        actions.remove(index)
    }

    pub fn refresh_now(&self) {
        let _ = self.refresh_now.try_send(());
    }

    pub fn iter_actions<'x>(&self, f: impl FnMut(&ActionToken) + 'x) {
        let mut actions = self.actions.lock();
        actions.retain(|v| !v.is_stopped());
        actions.iter().for_each(f);
    }

    pub fn cancel_action(&self, uuid: uuid::Uuid, force: bool) {
        let actions = self.actions.lock();
        let index = actions.iter().position(|a| a.uuid() == uuid).unwrap();
        if force {
            actions[index].cancle_force();
        } else {
            actions[index].cancel();
        }
    }
}

pub struct AsyncHandle<T> {
    #[cfg(not(target_arch = "wasm32"))]
    handle: tokio::task::JoinHandle<T>,

    #[cfg(target_arch = "wasm32")]
    cancellation: Arc<AtomicBool>,
    #[cfg(target_arch = "wasm32")]
    waiter: async_channel::Receiver<T>,
}

impl<T> AsyncHandle<T> {
    pub async fn await_handle(self) -> T {
        #[cfg(not(target_arch = "wasm32"))]
        return self.handle.await.unwrap();

        #[cfg(target_arch = "wasm32")]
        self.waiter.recv().await.unwrap()
    }

    pub fn abort(&self) {
        #[cfg(not(target_arch = "wasm32"))]
        return self.handle.abort();

        #[cfg(target_arch = "wasm32")]
        self.cancellation
            .store(true, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn is_finished(&self) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        return self.handle.is_finished();

        #[cfg(target_arch = "wasm32")]
        self.cancellation.load(std::sync::atomic::Ordering::Relaxed)
    }
}

#[pin_project::pin_project]
pub struct CancelableFuture<T> {
    #[pin]
    inner: T,
    cancel: Arc<AtomicBool>,
}

impl<T> Future for CancelableFuture<T>
where
    T: Future,
{
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.cancel.load(std::sync::atomic::Ordering::Relaxed) {
            return Poll::Ready(());
        }

        let project = self.project();
        project.inner.poll(cx).map(|_| {
            project
                .cancel
                .store(true, std::sync::atomic::Ordering::Relaxed);
        })
    }
}
