use std::{cell::RefCell, rc::Rc};

use dioxus::prelude::*;

#[derive(Clone)]
pub struct MountedDataStorge {
    mounted_data: Rc<RefCell<Option<Rc<MountedData>>>>,
}

impl MountedDataStorge {
    pub fn new() -> Self {
        Self {
            mounted_data: Rc::new(RefCell::new(None)),
        }
    }

    pub fn get(&self) -> Rc<MountedData> {
        self.mounted_data.borrow().as_ref().cloned().unwrap()
    }

    pub fn set(&self, mounted_data: Rc<MountedData>) {
        *self.mounted_data.borrow_mut() = Some(mounted_data);
    }
}

#[macro_export]
macro_rules! include_style {
    ($($key:ident: $file:expr),*) => {
        $(
            #[component]
            pub fn $key() -> Element {
                let text = include_str!($file);
                rsx! {
                    style { { text } }
                }
            }
        )*
    };
}
