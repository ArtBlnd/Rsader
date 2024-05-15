mod orderbook;
pub use orderbook::*;
mod dummy;
pub use dummy::*;

use dioxus::prelude::*;

/// A trait for all widgets
pub trait Widget {
    fn render(&self) -> Element;
}
