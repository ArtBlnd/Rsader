use std::sync::Arc;

mod ast;
mod currency;
mod global_context;
#[macro_use]
mod exchange;
mod vm;
#[macro_use]
mod window;
mod broadcast;
mod config;
mod ir;
mod utils;
mod websocket;

use exchange::{binance::Binance, bithumb::Bithumb, upbit::Upbit, Exchange, Exchanges};
use global_context::GlobalContext;
use iced::{executor, Application, Command, Element, Theme};
use window::dashboard;

pub struct Rsader {
    dashboard: dashboard::Dashboard,
}

impl Application for Rsader {
    type Executor = executor::Default;
    type Message = WindowMessage;
    type Theme = Theme;
    type Flags = ();

    fn new(_flags: Self::Flags) -> (Self, Command<Self::Message>) {
        let ex = Exchanges {
            upbit: Arc::new(Upbit::new()),
            binance: Arc::new(Binance::new()),
            bithumb: Arc::new(Bithumb::new()),
        };

        let (broadcaster, broadcast_rx) = broadcast::Broadcaster::new();
        let (global_ctx, refresh_now) = GlobalContext::new(ex.clone(), broadcaster.clone());
        ex.upbit.initialize(&global_ctx, broadcaster.clone());
        ex.binance.initialize(&global_ctx, broadcaster.clone());
        ex.bithumb.initialize(&global_ctx, broadcaster.clone());

        (
            Rsader {
                dashboard: dashboard::Dashboard::new(&global_ctx),
            },
            Command::batch([
                Command::run(broadcast_rx, |item| WindowMessage::Broadcast(item)),
                Command::run(refresh_now, |_| WindowMessage::RefreshNow),
            ]),
        )
    }

    fn title(&self) -> String {
        "Rsader".to_string()
    }

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        match message {
            WindowMessage::RefreshNow => {
                self.dashboard.refresh_now();
            }
            WindowMessage::Dashboard(message) => {
                return self.dashboard.update(message).map(WindowMessage::Dashboard);
            }
            WindowMessage::Broadcast(item) => {
                self.dashboard.broadcast(item);
            }
            WindowMessage::RuntimeExited => {
                std::process::exit(0);
            }
        }

        Command::none()
    }

    fn view(&self) -> Element<'_, Self::Message> {
        self.dashboard.view().map(WindowMessage::Dashboard)
    }

    fn theme(&self) -> Self::Theme {
        Theme::GruvboxDark
    }
}

#[derive(Clone, Debug)]
pub enum WindowMessage {
    RefreshNow,
    RuntimeExited,
    Dashboard(dashboard::Message),
    Broadcast(broadcast::Item),
}
