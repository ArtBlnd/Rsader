use std::time::Duration;

#[cfg(not(target_arch = "wasm32"))]
pub async fn sleep(duration: Duration) {
    tokio::time::sleep(duration).await;
}

#[cfg(any(target_arch = "wasm32"))]
pub async fn sleep(duration: Duration) {
    gloo_timers::future::sleep(duration).await;
}
