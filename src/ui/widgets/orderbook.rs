use std::sync::Arc;

use crate::{
    currency::Currency,
    exchange::{Exchange, RealtimeData},
    utils::broadcaster::Subscription,
};

use super::Widget;

use dioxus::prelude::*;

pub struct OrderbookWidget {
    pair: (Currency, Currency),
    subscription: Subscription<RealtimeData>,
}

impl OrderbookWidget {
    pub fn new<E>(pair: (Currency, Currency), exchange: &E) -> Self
    where
        E: Exchange + 'static,
    {
        Self {
            pair,
            subscription: exchange.subscribe(pair, None),
        }
    }
}

impl Widget for OrderbookWidget {
    fn render(&self) -> Element {
        rsx! {
            button {}
        }
    }
}
