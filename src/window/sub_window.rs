use iced::{Border, Color, Element, Shadow};

use crate::broadcast;

/// An object that represents a sub window of the main window.
/// which can be closed, opened, resized, moved, and minimized.
pub struct SubWindow<T> {
    uuid: uuid::Uuid,
    content: T,
}

impl<T> SubWindow<T> {
    pub fn new(uuid: uuid::Uuid, content: T) -> Self {
        Self { uuid, content }
    }
}

impl<T> SubWindow<T>
where
    T: SubWindowContent,
{
    pub fn uuid(&self) -> uuid::Uuid {
        self.uuid
    }

    pub fn title(&self) -> &str {
        self.content.title()
    }

    pub fn view(&self) -> Element<'_, T::Message> {
        let content = iced::widget::Container::new(self.content.view())
            .width(iced::Length::Fill)
            .height(iced::Length::Fill)
            .style(iced::widget::container::Appearance {
                text_color: Some(Color::WHITE),
                background: Some(iced::Background::Color(Color::from_rgb8(18, 18, 18))),
                border: Border {
                    ..Default::default()
                },
                shadow: Shadow::default(),
            });
        iced::widget::Container::new(content).into()
    }

    pub fn refresh_now(&mut self) {
        self.content.refresh_now();
    }

    pub fn update(&mut self, message: T::Message) {
        self.content.update(message);
    }

    pub fn broadcast(&mut self, item: broadcast::Item) {
        self.content.broadcast(item);
    }
}

enum SubwindowMessage<T> {
    Inner(T),
    Close,
}

pub trait SubWindowContent {
    type Message;

    fn title(&self) -> &str;
    fn view(&self) -> Element<'_, Self::Message>;
    fn refresh_now(&mut self);
    fn update(&mut self, message: Self::Message);
    fn broadcast(&mut self, item: broadcast::Item);
}
