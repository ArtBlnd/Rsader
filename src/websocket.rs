use async_channel::{Receiver as AsyncRx, Sender as AsyncTx};

use crate::utils::async_helpers;

/// Clonable websocket client implementation with auto-reconnect feature.
///
/// Note that the `Websocket` client only will recieve text messages.
/// If the binary messages are received, they will be silently ignored.
#[derive(Clone)]
pub struct Websocket {
    sender: AsyncTx<String>,
    recver: AsyncRx<String>,
}

impl Websocket {
    pub fn new(url: &str) -> Self {
        #[cfg(not(target_arch = "wasm32"))]
        let (sender, recver) = websocket_tokio::spawn_and_handle(url);
        #[cfg(target_arch = "wasm32")]
        let (sender, recver) = websocket_wasm::spawn_and_handle(url);

        Self { sender, recver }
    }

    pub async fn recv(&self) -> Option<String> {
        self.recver.recv().await.ok()
    }

    pub async fn send(&self, msg: &str) {
        self.sender.send(msg.to_string()).await.unwrap();
    }

    pub fn send_blocking(&self, msg: &str) {
        async_helpers::block_on(self.sender.send(msg.to_string())).unwrap()
    }
}

#[cfg(any(target_arch = "wasm32"))]
mod websocket_wasm {
    use async_channel::{Receiver as AsyncRx, Sender as AsyncTx};
    use futures::{SinkExt, StreamExt};
    use wasm_sockets::EventClient as WasmWebSocket;

    use crate::utils::async_helpers;

    pub(super) fn spawn_and_handle(url: &str) -> (AsyncTx<String>, AsyncRx<String>) {
        let (tx_sender, tx_recver) = async_channel::unbounded();
        let (rx_sender, rx_recver) = async_channel::unbounded();

        // TODO: Abort the future when the websocket is dropped.
        async_helpers::spawn(handler(url.to_string(), tx_recver, rx_sender));
        (tx_sender, rx_recver)
    }

    async fn handler(url: String, tx_recver: AsyncRx<String>, rx_sender: AsyncTx<String>) {
        let mut ws = WasmWebSocket::new(&url).unwrap();
        let tx_recver = tx_recver.clone();
        let rx_sender = rx_sender.clone();

        ws.set_on_message(Some(Box::new(
            move |client: &wasm_sockets::EventClient, message: wasm_sockets::Message| match message
            {
                wasm_sockets::Message::Text(text) => {
                    let _ = rx_sender.try_send(text);
                }
                wasm_sockets::Message::Binary(_) => {}
            },
        )));

        while let Ok(msg) = tx_recver.recv().await {
            ws.send_string(&msg).unwrap();
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod websocket_tokio {
    use async_channel::{Receiver as AsyncRx, Sender as AsyncTx};
    use futures::{SinkExt, StreamExt};
    use tokio::select;

    use crate::utils::async_helpers;

    pub(super) fn spawn_and_handle(url: &str) -> (AsyncTx<String>, AsyncRx<String>) {
        let (tx_sender, tx_recver) = async_channel::unbounded();
        let (rx_sender, rx_recver) = async_channel::unbounded();

        // TODO: Abort the future when the websocket is dropped.
        async_helpers::spawn(handler(url.to_string(), tx_recver, rx_sender));
        (tx_sender, rx_recver)
    }

    async fn handler(url: String, tx_recver: AsyncRx<String>, rx_sender: AsyncTx<String>) {
        use tokio_tungstenite::tungstenite::protocol::Message;

        // TODO: Add stop token to stop the loop.
        loop {
            let (ws_stream, _) = tokio_tungstenite::connect_async(url.clone()).await.unwrap();
            let (mut ws_sender, mut ws_recver) = ws_stream.split();

            let handle1 = {
                let tx_recver = tx_recver.clone();
                tokio::spawn(async move {
                    while let Ok(msg) = tx_recver.recv().await {
                        ws_sender.send(Message::Text(msg)).await.unwrap();
                    }
                })
            };

            let handle2 = {
                let rx_sender = rx_sender.clone();
                tokio::spawn(async move {
                    while let Some(msg) = ws_recver.next().await {
                        let msg = msg.unwrap();
                        if let Message::Text(text) = msg {
                            rx_sender.send(text).await.unwrap();
                        }
                    }
                })
            };

            // If any of the two handles are done, abort the other one.
            // And then, reconnect.
            let (ah1, ah2) = (handle1.abort_handle(), handle2.abort_handle());
            select! {
                _ = handle1 => { ah2.abort(); }
                _ = handle2 => { ah1.abort(); }
            }
        }
    }
}
