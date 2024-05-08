use iced::{widget::text_input::Id, Font, Length};
use iced_core::text::LineHeight;

pub struct Commander {
    buffer: String,
    id: Id,
}

impl Commander {
    pub fn new() -> Self {
        Commander {
            buffer: String::new(),
            id: Id::unique(),
        }
    }

    pub fn update(&mut self, message: Message) -> Option<String> {
        match message {
            Message::OnSubmit => Some(std::mem::take(&mut self.buffer)),
            Message::OnInput(input) => {
                self.buffer = input;
                None
            }
            Message::Clear => {
                self.clear();
                None
            }
        }
    }

    pub fn id(&self) -> &Id {
        &self.id
    }

    pub fn view(&self) -> iced::Element<'_, Message> {
        let font = Font::with_name("Space Mono");

        iced::widget::container(
            iced::widget::text_input("", &self.buffer)
                .id(self.id.clone())
                .font(font)
                .size(40.0)
                .line_height(LineHeight::Relative(1.2))
                .width(Length::Fixed(1000.0))
                .on_input(Message::OnInput)
                .on_submit(Message::OnSubmit),
        )
        .padding(200)
        .into()
    }

    pub fn clear(&mut self) {
        self.buffer.clear()
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    OnInput(String),
    OnSubmit,
    Clear,
}
