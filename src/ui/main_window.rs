use std::sync::Arc;

use dioxus::prelude::*;

use crate::currency::Currency;
use crate::exchange::binance::Binance;
use crate::exchange::bithumb::Bithumb;
use crate::exchange::upbit::Upbit;
use crate::exchange::{execute_if, Exchange};
use crate::ui::style::*;
use crate::ui::sub_window::{SubWindowEvent, SubWindowMgr, SubWindowMgrState};
use crate::ui::widgets::{Dummy, OrderbookWidget};
use crate::vm::exchange::install_exchange;
use crate::{include_style, select_ex};

#[component]
pub fn App() -> Element {
    // Keyboard shortcuts
    let keydown_events = use_signal(Vec::new);
    initialize_keydown_events(keydown_events);

    // Exchanges
    let upbit = use_hook(|| Arc::new(Upbit::new()));
    let binance = use_hook(|| Arc::new(Binance::new()));
    let bithumb = use_hook(|| Arc::new(Bithumb::new()));

    let ctx = MainWindowContext {
        keydown_events,
        upbit,
        binance,
        bithumb,
    };

    rsx! {
        // Basic element styles
        StylePrelude {}
        StyleMainWindow {}
        StyleFont {}
        StyleColor {}
        StyleButton { dark_mode: true }

        div { class: "main-window", width: "100%", height: "100%",
            MainWindow { ctx }
        }
    }
}

fn initialize_keydown_events(signal: Signal<Vec<(Key, Modifiers, Code)>>) {
    #[cfg(any(target_arch = "wasm32"))]
    mod wasm {
        use dioxus::prelude::*;
        use wasm_bindgen::prelude::*;

        pub fn init(mut signal: Signal<Vec<(Key, Modifiers, Code)>>) {
            let document = web_sys::window().unwrap().document().unwrap();
            let closure = Closure::wrap(Box::new(move |event: web_sys::Event| {
                let data = dioxus::prelude::KeyboardData::from(event);

                let key = data.key();
                let modifiers = data.modifiers();
                let code = data.code();

                signal.write().push((key, modifiers, code));
            }) as Box<dyn FnMut(_)>);

            document
                .add_event_listener_with_callback("keydown", closure.as_ref().unchecked_ref())
                .unwrap();
            closure.forget();
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    mod desktop {
        use dioxus::prelude::*;

        pub fn init(signal: Signal<Vec<(Key, Modifiers, Code)>>) {
            todo!()
        }
    }

    #[cfg(any(target_arch = "wasm32"))]
    wasm::init(signal);

    #[cfg(not(target_arch = "wasm32"))]
    desktop::init(signal);
}

#[derive(Clone)]
pub struct MainWindowContext {
    // Keyboard shortcuts
    keydown_events: Signal<Vec<(Key, Modifiers, Code)>>,

    // Exchanges
    upbit: Arc<Upbit>,
    binance: Arc<Binance>,
    bithumb: Arc<Bithumb>,
}

/// This is a dummy implementation of PartialEq for MainWindowContext
/// because we'll never modify the context.
impl PartialEq for MainWindowContext {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

#[component]
fn MainWindow(mut ctx: MainWindowContext) -> Element {
    let mut is_command_palette_open = use_signal(|| false);

    if !ctx.keydown_events.read().is_empty() {
        let events = ctx.keydown_events.take();
        for (key, modifiers, code) in events {
            match (key, modifiers, code) {
                (_, Modifiers::CONTROL, Code::Space) => *is_command_palette_open.write() = true,
                (_, _, Code::Escape) => *is_command_palette_open.write() = false,
                _ => {}
            }
        }
    }

    let mut commands = use_signal(String::new);
    if !commands.read().is_empty() {
        if let Some(command) = Command::parse(&commands.take()) {
            match command {
                Command::Orderbook(ex_name, (base, quote)) => {
                    if let Some(widget) = select_ex!(ctx, ex_name, |exchange| {
                        OrderbookWidget::new((base, quote), exchange)
                    }) {
                        SubWindowMgrState::open(widget.into());
                    }
                }
            }

            *is_command_palette_open.write() = false;
        }
    }

    rsx! {
        if *is_command_palette_open.peek() {
            CommandPalette { commands }
        }

        SubWindowMgr {}
    }
}

#[derive(Debug)]
enum Command {
    Orderbook(String, (Currency, Currency)),
}

impl Command {
    pub fn parse(command: &str) -> Option<Command> {
        let command = command.trim().split_whitespace().collect::<Vec<_>>();
        match command.as_slice() {
            ["orderbook", ex_name, pair] => {
                let mut pair = pair.split('-');
                let base = pair.next()?.to_uppercase().parse().ok()?;
                let quote = pair.next()?.to_uppercase().parse().ok()?;

                Some(Command::Orderbook(ex_name.to_string(), (base, quote)))
            }
            _ => None,
        }
    }
}

#[component]
fn CommandPalette(commands: Signal<String>) -> Element {
    include_style!(
        CommandPaletteStyle:
        "../../resources/CommandPaletteStyle.css"
    );

    rsx! {
        CommandPaletteStyle {}
        div {
            role: "dialog",
            aria_modal: "true",
            class: "palette",
            style: "z-index: 999",
            div { class: "wrapper",
                div { class: "contents",
                    div { class: "search",
                        input {
                            class: "input",
                            r#type: "text",
                            placeholder: "Command...",
                            spellcheck: "false",

                            oninput: move |input| {
                                *commands.write() = input.value();
                            },

                            onmounted: move |input| { async move {
                                input.set_focus(true).await;
                            }
                            },
                        }
                    }
                }
            }
        }
    }
}
