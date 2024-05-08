use iced::{widget::Column, Element};

use crate::{broadcast, global_context::GlobalContext};

use super::sub_window::SubWindowContent;

pub struct ActionListWindow {
    global_ctx: GlobalContext,
}

impl ActionListWindow {
    pub fn new(global_ctx: &GlobalContext) -> Self {
        Self {
            global_ctx: global_ctx.clone(),
        }
    }
}

impl SubWindowContent for ActionListWindow {
    type Message = Message;

    fn title(&self) -> &str {
        "Action List"
    }

    fn view(&self) -> Element<'_, Self::Message> {
        let mut rows = Vec::new();
        self.global_ctx.iter_actions(|action| {
            rows.push(
                iced::widget::row![
                    iced::widget::Text::new(action.uuid().to_string())
                        .vertical_alignment(iced::alignment::Vertical::Center),
                    iced::widget::Text::new(action.state())
                        .vertical_alignment(iced::alignment::Vertical::Center),
                ]
                .spacing(5),
            );
        });

        let mut columns = Column::new().align_items(iced::Alignment::Start);
        for row in rows {
            columns = columns.push(row);
        }

        columns.into()
    }

    fn refresh_now(&mut self) {}

    fn update(&mut self, message: Self::Message) {
        match message {
            Message::Cancel(uuid) => {
                self.global_ctx.cancel_action(uuid, false);
            }
            Message::CancelForce(uuid) => {
                self.global_ctx.cancel_action(uuid, true);
            }
        }
    }

    fn broadcast(&mut self, item: broadcast::Item) {}
}

#[derive(Clone, Debug)]
pub enum Message {
    Cancel(uuid::Uuid),
    CancelForce(uuid::Uuid),
}
