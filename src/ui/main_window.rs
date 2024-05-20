use dioxus::prelude::*;

use crate::ui::style::*;
use crate::ui::sub_window::{SubWindowEvent, SubWindowMgr, SubWindowMgrState};
use crate::ui::widgets::Dummy;

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
    rsx! {
        button {
            class: "rbutton",
            onmousedown: move |_| {
                SubWindowMgrState::send(SubWindowEvent::WindowCreation(Dummy::new().into()));
            },
            "Add SubWindow"
        }

        SubWindowMgr {}
    }
}
