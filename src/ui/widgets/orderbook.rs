use std::sync::Arc;

use crate::{
    currency::Currency,
    exchange::{Exchange, RealtimeData},
    utils::{broadcaster::Subscription, flag::Flag},
};

use super::Widget;

use crate::utils::Decimal;
use dioxus::prelude::*;
use unwrap_let::unwrap_let;

pub struct OrderbookWidget {
    pair: (Currency, Currency),
    exchange_name: String,
    subscription: Subscription<RealtimeData>,

    need_rerender: Flag<bool>,
}

impl OrderbookWidget {
    pub fn new<E>(pair: (Currency, Currency), exchange: Arc<E>) -> Self
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
        let subscription = self.subscription.clone();
        let mut data = use_resource(move || {
            let subscription = subscription.clone();
            async move {
                loop {
                    if let RealtimeData::Orderbook(value) = subscription.recv().await {
                        return value;
                    }
                }
            }
        });

        if data.finished() {
            data.restart();
        }
        let data = data.read();
        let orderbook = data.as_ref()?;

        rsx! {
            OrderbookBarStyle {}
            div { class: "color-1",
                ul { style: "list-style: none;  display: flex; flex-direction: column; padding: 0; margin: 0;",
                    for ask in orderbook.asks.iter().take(4).rev() {
                        OrderbookBar { is_green: false, price: ask.price, amount: ask.amount }
                    }
                    for bid in orderbook.bids.iter().take(4) {
                        OrderbookBar { is_green: true, price: bid.price, amount: bid.amount }
                    }
                }
            }
        }
    }

    fn name(&self) -> String {
        format!("{} {}-{}", self.exchange_name, self.pair.0, self.pair.1)
    }

    fn is_changed_after_render(&self) -> bool {
        self.need_rerender.get().unwrap_or_default()
    }
}

#[component]
fn OrderbookBarStyle() -> Element {
    let text = r#"
    .bar-height {
        height: 30px;
    }
    .orderbook-bar {
        position: absolute;
        width: 100%;
        z-index: 1;
    }
    .color-obb-green {
        background-color: #152f1e;
    }
    .color-obb-red {
        background-color: #361b22;
    }
    .orderbook-bar-font {
        position: relative;
        z-index: 2;
        justify-content: space-between;
        vertical-align: middle;
        flex 1 0; 
        align-items: center; 
        display: flex; 
        overflow: hidden;
    }
    .color-obb-font-green {
        color: #228a44
    }
    .color-obb-font-red {
        color: #a63654
    }
    "#;
    rsx! {
        style { { text } }
    }
}

#[component]
fn OrderbookBar(is_green: bool, price: Decimal, amount: Decimal) -> Element {
    let obb_font_color = if is_green {
        "color-obb-font-green"
    } else {
        "color-obb-font-red"
    };

    let obb_color = if is_green {
        "color-obb-green"
    } else {
        "color-obb-red"
    };

    rsx! {
        li {
            class: "bar-height",
            style: "display:flex; align-items: center; justify-content: space-between",
            div { class: "bar-height orderbook-bar {obb_color}" }
            span {
                width: "100%",
                class: "bar-height orderbook-bar-font font2 {obb_font_color}",
                style: "padding-left: 10px; text-align: left;",
                "{price}"
            }
            span {
                width: "100%",
                class: "bar-height orderbook-bar-font font2 {obb_font_color}",
                style: "text-align: right; padding-right: 10px; ",
                "{amount}"
            }
        }
    }
}
