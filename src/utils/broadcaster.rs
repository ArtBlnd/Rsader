use std::sync::Arc;

use async_channel::{Receiver as Rx, Sender as Tx};
use parking_lot::Mutex;

#[derive(Clone)]
pub struct Broadcaster<T> {
    subscriptions: Arc<Mutex<Vec<Tx<T>>>>,
}

impl<T> Broadcaster<T>
where
    T: Clone,
{
    pub fn new() -> Self {
        Self {
            subscriptions: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Broadcasts data to all subscribers.
    /// If a subscriber is no longer listening, it is removed from the list.
    pub fn broadcast(&self, data: T) {
        let mut subscriptions = self.subscriptions.lock();
        subscriptions.retain(|tx| tx.try_send(data.clone()).is_ok());
    }

    /// Subscribes to the broadcaster.
    /// Returns a receiver that will receive all data broadcasted.
    pub fn subscribe(&self) -> Subscription<T> {
        let mut subscriptions = self.subscriptions.lock();
        let (tx, rx) = async_channel::unbounded();
        subscriptions.push(tx);
        Subscription { receiver: rx }
    }
}

pub struct Subscription<T> {
    receiver: Rx<T>,
}

impl<T> Subscription<T> {
    /// Receives data from the broadcaster.
    /// The function will panic if the broadcaster has been dropped.
    pub async fn recv(&self) -> T {
        self.receiver.recv().await.unwrap()
    }

    /// Clears the subscription buffer.
    /// This is useful if you want to ignore old data.
    pub fn clear(&self) {
        let len = self.receiver.len();
        for _ in 0..len {
            let _ = self.receiver.try_recv();
        }
    }
}
