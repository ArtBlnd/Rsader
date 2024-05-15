use dioxus::prelude::*;

use crate::ui::style::*;
use crate::ui::sub_window::{SubWindowMgr, SubWindowMgrState};
use crate::ui::widgets::Dummy;

#[component]
pub fn App() -> Element {
    rsx! {
        // Basic element styles
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
    let mut state = use_signal(|| SubWindowMgrState::new());

    rsx! {
        button {
            class: "rbutton",
            onmousedown: move |_| {
                state.write().append(Dummy::new());
            },
            "Add SubWindow"
        }

        SubWindowMgr { state }
    }
}
