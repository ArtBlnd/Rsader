use std::{
    pin::Pin,
    sync::{atomic::AtomicBool, Arc},
    task::{Context, Poll},
};

use crate::{
    broadcast::{self, Broadcaster},
    exchange::Exchanges,
};

use futures::{Future, Stream};
use parking_lot::Mutex;

pub struct GlobalContext {
    pub ex: Exchanges,
}

impl GlobalContext {
    pub fn get() -> &'static GlobalContext {
        todo!()
    }
}
