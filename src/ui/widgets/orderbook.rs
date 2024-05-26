use std::sync::Arc;

use crate::{
    currency::Currency,
    dec,
    exchange::{Exchange, RealtimeData},
    utils::{broadcaster::Subscription, flag::Flag},
};

use super::Widget;

use crate::utils::Decimal;
use dioxus::prelude::*;

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
        let pair = self.pair.clone();

        let mut data = use_resource(move || {
            let subscription = subscription.clone();
            let pair = pair.clone();
            async move {
                loop {
                    if let RealtimeData::Orderbook(value) = subscription.recv().await {
                        if value.pair == pair {
                            return value;
                        }
                    }
                }
            }
        });

        if data.finished() {
            data.restart();
        }

        let data = data.read();
        let orderbook = data.as_ref()?;

        let min_length = orderbook.asks.len().min(orderbook.bids.len());
        let asks = orderbook.asks.iter().take(min_length).rev();
        let bids = orderbook.bids.iter().take(min_length);

        let max_ask = asks.clone().map(|x| x.amount).max();
        let max_bid = bids.clone().map(|x| x.amount).max();
        let max = max_ask.max(max_bid)?;

        rsx! {
            OrderbookBarStyle {}
            ul { style: "list-style: none;  display: flex; flex-direction: column; padding: 0; margin: 0; align-content: center;",
                for ask in orderbook.asks.iter().take(min_length).rev() {
                    OrderbookBar {
                        is_green: false,
                        price: ask.price,
                        amount: ask.amount,
                        ratio: ask.amount / max
                    }
                }
                for bid in orderbook.bids.iter().take(min_length) {
                    OrderbookBar { is_green: true, price: bid.price, amount: bid.amount, ratio: bid.amount / max }
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
        right: 0;
        z-index: 1;
    }
    .color-obb-green {
        background-color: #152f1e;
    }
    .color-obb-red {
        background-color: #361b22;
    }
    .orderbook-bar-text {
        line-height: 30px;
        z-index: 2;
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
fn OrderbookBar(is_green: bool, price: Decimal, amount: Decimal, ratio: Decimal) -> Element {
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

    let ratio = ratio * dec!(100);

    rsx! {
        li {
            class: "bar-height",
            style: "display:flex; align-items: center; justify-content: space-between;",
            div {
                class: "bar-height orderbook-bar {obb_color}",
                style: "transition: width 0.5s;",
                width: "{ratio}%"
            }
            span {
                width: "100%",
                class: "bar-height orderbook-bar-text font2 {obb_font_color}",
                style: "padding-left: 10px; text-align: left;",
                "{price}"
            }
            span {
                width: "100%",
                class: "bar-height orderbook-bar-text font2 {obb_font_color}",
                style: "text-align: right; padding-right: 10px; ",
                "{amount}"
            }
        }
    }
}
