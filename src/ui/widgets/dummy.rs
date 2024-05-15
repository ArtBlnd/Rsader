use super::Widget;

use dioxus::prelude::*;

pub struct Dummy {
    uuid: uuid::Uuid,
}

impl Dummy {
    pub fn new() -> Self {
        Self {
            uuid: uuid::Uuid::new_v4(),
        }
    }
}

impl Widget for Dummy {
    fn render(&self) -> Element {
        let uuid = self.uuid.to_string();
        rsx! {
            div { class: "font-color-w font1", "{uuid}" }
        }
    }
}