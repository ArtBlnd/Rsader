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
        tracing::info!("Rendering Dummy widget");
        let uuid = self.uuid.to_string();
        rsx! {
            div { class: "font-color-main font1", "{uuid}" }
        }
    }

    fn name(&self) -> String {
        "Dummy".to_string()
    }

    fn is_changed_after_render(&self) -> bool {
        false
    }
}
