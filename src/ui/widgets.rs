mod orderbook;
use std::{ops::Deref, sync::Arc};

pub use orderbook::*;
mod dummy;
pub use dummy::*;

use dioxus::prelude::*;

/// A trait for all widgets
pub trait Widget {
    fn render(&self) -> Element;
    fn name(&self) -> String;

    /// Returns true if the widget should be re-rendered after the render phase
    fn is_changed_after_render(&self) -> bool {
        // We assume that the widget is changed after render by default
        true
    }
}

#[derive(Clone)]
pub struct BoxedWidget(Arc<dyn Widget + Send + Sync + 'static>);

impl<W> From<W> for BoxedWidget
where
    W: Widget + Send + Sync + 'static,
{
    fn from(widget: W) -> Self {
        BoxedWidget(Arc::new(widget))
    }
}

impl PartialEq for BoxedWidget {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0) && !self.0.is_changed_after_render()
    }
}

impl AsRef<dyn Widget> for BoxedWidget {
    fn as_ref(&self) -> &(dyn Widget + 'static) {
        self.0.as_ref()
    }
}

impl Deref for BoxedWidget {
    type Target = dyn Widget;

    fn deref(&self) -> &Self::Target {
        self.0.as_ref()
    }
}

#[component]
pub fn WidgetElement(widget: BoxedWidget) -> Element {
    widget.0.render()
}
