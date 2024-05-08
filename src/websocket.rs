#[cfg(any(target_arch = "wasm32"))]
mod websocket_wasm {
    use futures::lock::Mutex;
    use wasm_sockets::EventClient as WasmWebSocket;

    pub struct Websocket {
        inner: Mutex<Option<WasmWebSocket>>,
        recv: async_channel::Receiver<String>,
        send: async_channel::Sender<String>,
    }

    impl Websocket {
        pub fn new() -> Self {
            let (send, recv) = async_channel::unbounded();
            Self {
                inner: Mutex::new(None),
                recv,
                send,
            }
        }

        pub async fn is_connected(&self) -> bool {
            self.inner.lock().await.is_some()
        }

        pub async fn connect(&self, url: &str) {
            let mut ws = WasmWebSocket::new(url).unwrap();
            let mut send = self.send.clone();
            ws.set_on_message(Some(Box::new(
                move |client: &wasm_sockets::EventClient, message: wasm_sockets::Message| {
                    match message {
                        wasm_sockets::Message::Text(text) => {
                            let _ = send.try_send(text);
                        }
                        wasm_sockets::Message::Binary(_) => {}
                    }
                },
            )));

            let mut v = self.inner.lock().await;
            *v = Some(ws);
        }

        pub async fn recv(&self) -> Option<String> {
            self.recv.recv().await.ok()
        }

        pub async fn send(&self, msg: &str) {
            self.inner
                .lock()
                .await
                .as_ref()
                .unwrap()
                .send_string(msg)
                .unwrap();
        }
    }
}

#[cfg(any(target_arch = "wasm32"))]
pub use websocket_wasm::Websocket;

#[cfg(not(target_arch = "wasm32"))]
mod websocket_tokio {
    use futures::{
        lock::Mutex,
        stream::{SplitSink, SplitStream},
        SinkExt, StreamExt,
    };
    use tokio::net::TcpStream;
    use tokio_tungstenite::{tungstenite::Message, MaybeTlsStream, WebSocketStream};

    pub struct Websocket {
        sender: Mutex<Option<SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>>>,
        recver: Mutex<Option<SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>>>,
    }

    impl Websocket {
        pub fn new() -> Self {
            Self {
                sender: Mutex::new(None),
                recver: Mutex::new(None),
            }
        }

        pub async fn connect(&self, url: &str) {
            let (ws, _) = tokio_tungstenite::connect_async(url)
                .await
                .expect("Failed to connect");

            let (sender, recver) = ws.split();
            *self.sender.lock().await = Some(sender);
            *self.recver.lock().await = Some(recver);
        }

        pub async fn recv(&self) -> Option<String> {
            let mut v = self.recver.lock().await;
            let ws = v.as_mut()?;

            let Some(Ok(msg)) = ws.next().await else {
                v.take();
                return None;
            };

            Some(msg.to_string())
        }

        pub async fn send(&self, msg: &str) {
            let mut v = self.sender.lock().await;
            let ws = v.as_mut().expect("Not connected yet");
            ws.send(msg.into()).await.expect("Failed to send message");
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub use websocket_tokio::Websocket;
