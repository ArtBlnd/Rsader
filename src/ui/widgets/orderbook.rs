use std::sync::Arc;

use crate::{
    currency::Currency,
    exchange::{Exchange, RealtimeData},
    utils::{broadcaster::Subscription, flag::Flag},
};

use super::Widget;

use dioxus::prelude::*;

pub struct OrderbookWidget {
    pair: (Currency, Currency),
    exchange_name: String,
    subscription: Subscription<RealtimeData>,

    need_rerender: Flag<bool>,
}

impl OrderbookWidget {
    pub fn new<E>(pair: (Currency, Currency), exchange: &E) -> Self
    where
        E: Exchange + 'static,
    {
        Self {
            pair,
            exchange_name: E::NAME.to_string(),
            subscription: exchange.subscribe(pair, None),

            need_rerender: Flag::new(),
        }
    }
}

impl Widget for OrderbookWidget {
    fn render(&self) -> Element {
        rsx! {
            button {}
        }
    }

    fn name(&self) -> String {
        format!("{} {}-{}", self.exchange_name, self.pair.0, self.pair.1)
    }

    fn is_changed_after_render(&self) -> bool {
        self.need_rerender.get().unwrap_or_default()
    }
}
