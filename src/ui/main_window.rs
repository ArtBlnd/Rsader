use std::sync::Arc;

use dioxus::prelude::*;

use crate::currency::Currency;
use crate::exchange::binance::Binance;
use crate::exchange::upbit::Upbit;
use crate::ui::style::*;
use crate::ui::sub_window::{SubWindowEvent, SubWindowMgr, SubWindowMgrState};
use crate::ui::widgets::{Dummy, OrderbookWidget};

#[component]
pub fn App() -> Element {
    rsx! {
        // Basic element styles
        StylePrelude {}
        StyleMainWindow {}
        StyleFont {}
        StyleColor {}
        StyleButton { dark_mode: true }

        // Style for widgets

        div { class: "main-window", width: "100%", height: "100%", MainWindow {} }
    }
}

#[component]
pub fn MainWindow() -> Element {
    let upbit = use_hook(|| Arc::new(Upbit::new()));

    rsx! {
        button {
            class: "rbutton",
            onmousedown: move |_| {
                let upbit = upbit.clone();
                SubWindowMgrState::send(SubWindowEvent::WindowCreation(OrderbookWidget::new((Currency::BTC, Currency::KRW), upbit).into()));
            },
            "Add SubWindow"
        }

        SubWindowMgr {}
    }
}
