use std::{
    pin::Pin,
    sync::{atomic::AtomicBool, Arc},
    task::{Context, Poll},
    time::Duration,
};

use futures::Future;

#[cfg(not(target_arch = "wasm32"))]
pub async fn sleep(duration: Duration) {
    tokio::time::sleep(duration).await;
}

#[cfg(any(target_arch = "wasm32"))]
pub async fn sleep(duration: Duration) {
    gloo_timers::future::sleep(duration).await;
}

#[cfg(not(target_arch = "wasm32"))]
static RUNTIME: once_cell::sync::Lazy<tokio::runtime::Runtime> = once_cell::sync::Lazy::new(|| {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
});

#[cfg(not(target_arch = "wasm32"))]
pub fn spawn<T>(future: T) -> AsyncHandle<T::Output>
where
    T: Future + Send + 'static,
    T::Output: Send + 'static,
{
    let join_handle = RUNTIME.spawn(future);
    AsyncHandle {
        handle: join_handle,
    }
}

#[cfg(any(target_arch = "wasm32"))]
pub fn spawn<T>(future: T) -> AsyncHandle<T::Output>
where
    T: Future + 'static,
    T::Output: 'static,
{
    use async_channel::unbounded;
    let cancellation = Arc::new(AtomicBool::new(false));

    let (sender, receiver) = unbounded();
    let cancel = cancellation.clone();
    let future = async move {
        let result = future.await;
        let _ = sender.send(result).await;
    };

    wasm_bindgen_futures::spawn_local(CancelableFuture {
        inner: future,
        cancel,
    });

    AsyncHandle {
        cancellation,
        waiter: receiver,
    }
}

pub fn block_on<T>(future: T) -> T::Output
where
    T: Future,
{
    futures::executor::block_on(future)
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
