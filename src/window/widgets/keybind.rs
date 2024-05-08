use iced::{
    keyboard::{Key, Modifiers},
    Element, Event,
};
use iced_core::Widget;

pub struct Keybind {
    key: Key,
    modifier: Modifiers,
}

impl Keybind {
    pub fn new(key: Key, modifier: Modifiers) -> Self {
        Self { key, modifier }
    }

    pub fn key(&self) -> &Key {
        &self.key
    }

    pub fn modifier(&self) -> Modifiers {
        self.modifier
    }
}

pub struct Shortcut<'a, Message, Theme, Renderer> {
    content: Element<'a, Message, Theme, Renderer>,
    keybind: Keybind,
    on_press: Box<dyn Fn() -> Message + 'a>,
}

impl<'a, Message, Theme, Renderer> Shortcut<'a, Message, Theme, Renderer> {
    pub fn from_keybind(
        content: impl Into<Element<'a, Message, Theme, Renderer>>,
        keybind: Keybind,
        on_press: impl Fn() -> Message + 'a,
    ) -> Self {
        Self {
            content: content.into(),
            keybind,
            on_press: Box::new(on_press),
        }
    }
}

impl<'a, Theme, Message, Renderer> Widget<Message, Theme, Renderer>
    for Shortcut<'a, Message, Theme, Renderer>
where
    Renderer: iced_core::Renderer,
{
    fn size(&self) -> iced::Size<iced::Length> {
        self.content.as_widget().size()
    }

    fn layout(
        &self,
        tree: &mut iced_core::widget::Tree,
        renderer: &Renderer,
        limits: &iced_core::layout::Limits,
    ) -> iced_core::layout::Node {
        self.content.as_widget().layout(tree, renderer, limits)
    }

    fn draw(
        &self,
        tree: &iced_core::widget::Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &iced_core::renderer::Style,
        layout: iced_core::Layout<'_>,
        cursor: iced_core::mouse::Cursor,
        viewport: &iced::Rectangle,
    ) {
        renderer.with_layer(layout.bounds(), |v| {
            self.content
                .as_widget()
                .draw(tree, v, theme, style, layout, cursor, viewport);
        });
    }

    fn size_hint(&self) -> iced::Size<iced::Length> {
        self.content.as_widget().size_hint()
    }

    fn tag(&self) -> iced_core::widget::tree::Tag {
        self.content.as_widget().tag()
    }

    fn state(&self) -> iced_core::widget::tree::State {
        self.content.as_widget().state()
    }

    fn children(&self) -> Vec<iced_core::widget::Tree> {
        self.content.as_widget().children()
    }

    fn diff(&self, tree: &mut iced_core::widget::Tree) {
        self.content.as_widget().diff(tree);
    }

    fn operate(
        &self,
        _state: &mut iced_core::widget::Tree,
        _layout: iced_core::Layout<'_>,
        _renderer: &Renderer,
        _operation: &mut dyn iced_core::widget::Operation<Message>,
    ) {
        self.content
            .as_widget()
            .operate(_state, _layout, _renderer, _operation);
    }

    fn on_event(
        &mut self,
        _state: &mut iced_core::widget::Tree,
        event: iced::Event,
        _layout: iced_core::Layout<'_>,
        _cursor: iced_core::mouse::Cursor,
        _renderer: &Renderer,
        _clipboard: &mut dyn iced_core::Clipboard,
        shell: &mut iced_core::Shell<'_, Message>,
        _viewport: &iced::Rectangle,
    ) -> iced_core::event::Status {
        if let Event::Keyboard(iced::keyboard::Event::KeyPressed { key, modifiers, .. }) = &event {
            if key == self.keybind.key() && modifiers == &self.keybind.modifier() {
                shell.invalidate_widgets();
                shell.publish((self.on_press)());
                return iced_core::event::Status::Captured;
            }
        }

        self.content.as_widget_mut().on_event(
            _state, event, _layout, _cursor, _renderer, _clipboard, shell, _viewport,
        )
    }

    fn mouse_interaction(
        &self,
        _state: &iced_core::widget::Tree,
        _layout: iced_core::Layout<'_>,
        _cursor: iced_core::mouse::Cursor,
        _viewport: &iced::Rectangle,
        _renderer: &Renderer,
    ) -> iced_core::mouse::Interaction {
        self.content
            .as_widget()
            .mouse_interaction(_state, _layout, _cursor, _viewport, _renderer)
    }
}

impl<'a, Message, Theme, Renderer> From<Shortcut<'a, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Theme: 'a,
    Message: 'a,
    Renderer: 'a + iced_core::Renderer,
{
    fn from(shortcut: Shortcut<'a, Message, Theme, Renderer>) -> Self {
        Element::new(shortcut)
    }
}
