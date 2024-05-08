use std::fmt::{self, Debug, Formatter};
use std::{any::Any, sync::Arc};

use futures::Stream;

#[derive(Clone)]
pub struct Item(Option<BroadcastFrom>, Arc<dyn Any + Send + Sync>);

impl Item {
    pub fn as_ref<T: 'static>(&self) -> Option<&T> {
        self.1.downcast_ref()
    }

    pub fn from(&self) -> Option<&BroadcastFrom> {
        self.0.as_ref()
    }
}

impl Debug for Item {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Item").field(&"dyn Any").finish()
    }
}

#[derive(Clone)]
pub struct Broadcaster {
    tx: async_channel::Sender<Item>,
}

impl Broadcaster {
    pub fn new() -> (Self, impl Stream<Item = Item>) {
        let (tx, rx) = async_channel::unbounded();
        (
            Self { tx },
            futures::stream::unfold(rx, |rx| async move { Some((rx.recv().await.unwrap(), rx)) }),
        )
    }

    pub fn broadcast<T>(&self, from: Option<BroadcastFrom>, item: T)
    where
        T: Send + Sync + 'static,
    {
        let item = Item(from, Arc::new(item));
        self.tx.try_send(item).unwrap();
    }
}

#[derive(Clone, Debug)]
pub enum BroadcastFrom {
    Exchange(&'static str),
}
