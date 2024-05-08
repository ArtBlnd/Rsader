use crate::{
    broadcast::{self, BroadcastFrom},
    currency::Currency,
    exchange::{Exchange, Orderbook},
    global_context::GlobalContext,
};

use super::{canvas::OrderbookProgram, sub_window::SubWindowContent};

pub struct OrderbookWindow {
    title: String,
    pair: (Currency, Currency),
    exchange_name: String,

    current_orderbook: Option<Orderbook>,
}

impl OrderbookWindow {
    pub fn new(
        pair: (Currency, Currency),
        exchange_name: &str,
        global_ctx: &GlobalContext,
    ) -> Self {
        select_ex!(exchange_name, global_ctx.ex(), ex, {
            ex.subscribe(pair, None);
        });

        Self {
            title: format!("Orderbook({}): {}-{}", exchange_name, pair.0, pair.1),
            pair,
            exchange_name: exchange_name.to_string(),
            current_orderbook: None,
        }
    }
}

impl SubWindowContent for OrderbookWindow {
    type Message = Message;

    fn title(&self) -> &str {
        &self.title
    }

    fn view(&self) -> iced::Element<'_, Self::Message> {
        if let Some(orderbook) = &self.current_orderbook {
            iced::widget::Canvas::new(OrderbookProgram::new(orderbook.clone()))
                .width(iced::Length::Fill)
                .height(iced::Length::Fill)
                .into()
        } else {
            iced::widget::Text::new("No orderbook data").into()
        }
    }

    fn refresh_now(&mut self) {}
    fn update(&mut self, _message: Self::Message) {}

    fn broadcast(&mut self, item: broadcast::Item) {
        let Some(BroadcastFrom::Exchange(exchange)) = item.from() else {
            return;
        };

        if exchange != &self.exchange_name {
            return;
        }

        if let Some(orderbook) = item.as_ref::<Orderbook>() {
            if orderbook.pair == self.pair {
                self.current_orderbook = Some(orderbook.clone());
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    None,
}
