use std::collections::HashMap;
use std::rc::Rc;

use super::widgets::Widget;

use dioxus::prelude::*;
use once_cell::sync::OnceCell;

pub struct SubWindow {
    uuid: uuid::Uuid,
    name: String,
    mount_data: Rc<OnceCell<Rc<MountedData>>>,

    widget: Box<dyn Widget>,
}

impl SubWindow {
    pub fn new<T, W>(name: T, widget: W) -> Self
    where
        T: Into<String>,
        W: Widget + 'static,
    {
        Self {
            uuid: uuid::Uuid::new_v4(),
            name: name.into(),
            mount_data: Rc::new(OnceCell::new()),

            widget: Box::new(widget),
        }
    }
}

impl SubWindow {
    fn render(&self, mut signal: Signal<Vec<SubWindowEvent>>) -> Element {
        let uuid = self.uuid;
        let name = self.name.clone();

        let mount_data = self.mount_data.clone();

        rsx! {
            div {
                class: "color-2 pane",
                onmousedown: move |_| signal.write().push(SubWindowEvent::Focus(uuid)),
                onmounted: move |data| {
                    let _ = mount_data.set(data.data().clone());
                },

                SubWindowBar { name, uuid, signal }
                { self.widget.render() }
            }
        }
    }

    pub async fn position(&self) -> (f64, f64) {
        let mount_data = self.mount_data.get().unwrap();
        let rect = mount_data.get_client_rect().await.unwrap();

        (rect.origin.x, rect.origin.y)
    }

    pub async fn size(&self) -> (f64, f64) {
        let mount_data = self.mount_data.get().unwrap();
        let rect = mount_data.get_client_rect().await.unwrap();

        (rect.size.width, rect.size.height)
    }
}

#[component]
fn SubWindowBar(name: String, uuid: uuid::Uuid, signal: Signal<Vec<SubWindowEvent>>) -> Element {
    rsx! {
        div {
            onmousedown: move |_| signal.write().push(SubWindowEvent::MouseDown(uuid)),
            onmouseup: move |e| {
                e.stop_propagation();
                let coords = e.page_coordinates();
                signal.write().push(SubWindowEvent::MouseUp(uuid, coords.x, coords.y))
            },

            div { class: "color-2 subwindow-bar-title", {name} }
            div {
                class: "font-color-w",
                onmousedown: move |e| {
                    e.stop_propagation();
                    signal.write().push(SubWindowEvent::Close(uuid));
                },
                "X"
            }
        }
    }
}

#[component]
fn StylePrelude() -> Element {
    let text = r#"
.panes {
    touch-action: none;
    display: grid;
}
.pane {
    position: relative;
    overflow: hidden;
}
.splitter-h {
    position: absolute;
    right: 0;
    z-index: 3;
    background: #000;
    cursor: ew-resize;
    top: 0;
    width: 8px;
}
.splitter-v {
    cursor: ns-resize;
    left: 0;
    height: 8px;
    position: absolute;
    right: 0;
    z-index: 3;
    background: #000;
}"#;

    rsx! {
        style { {text} }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum SubWindowEvent {
    MouseDown(uuid::Uuid),
    MouseUp(uuid::Uuid, f64, f64),
    MouseMove(uuid::Uuid, f64, f64),
    Close(uuid::Uuid),
    Focus(uuid::Uuid),
}

pub struct SubWindowMgrState {
    windows: HashMap<uuid::Uuid, SubWindow>,
    root: VSplit, // Tree of splits

    focused: uuid::Uuid,
    dragging: Option<uuid::Uuid>,
}

impl SubWindowMgrState {
    pub fn new() -> Self {
        Self {
            windows: HashMap::new(),
            root: VSplit::new(),

            focused: uuid::Uuid::nil(),
            dragging: None,
        }
    }

    pub fn append<W>(&mut self, widget: W)
    where
        W: Widget + 'static,
    {
        let window = SubWindow::new("Untitled", widget);
        let window_uuid = window.uuid;

        if self.windows.is_empty() {
            // If there are no windows, then this is the first window
            // So, set it as the focused window
            self.focused = window_uuid;
            self.root.append(window_uuid);
        } else {
            // If there are windows, then append it to the split tree
            self.root
                .split_append(self.focused, window_uuid, SplitSide::Bottom);
        }

        self.windows.insert(window_uuid, window);
    }

    pub fn remove(&mut self, uuid: uuid::Uuid) {
        self.root.remove(uuid);
        self.windows.remove(&uuid);

        // If the focused window is removed, then set the focused window to the first window
        if self.focused == uuid {
            self.focused = self.root.first().unwrap_or(uuid::Uuid::nil());
        }
    }

    pub fn render(&self, event_queue: Signal<Vec<SubWindowEvent>>) -> Element {
        self.root.render_element(&self.windows, event_queue)
    }

    pub fn dispatch_mouse_down(&mut self, uuid: uuid::Uuid) {
        self.dragging = Some(uuid);
    }

    pub async fn dispatch_mouse_up(&mut self, id: uuid::Uuid, x: f64, y: f64) {
        let Some(dragged) = self.dragging.take() else {
            return;
        };

        assert!(self.windows.contains_key(&id));
        assert!(self.windows.contains_key(&dragged));

        if dragged == id {
            return;
        }

        let (target_x, target_y) = self.windows[&id].position().await;
        let (target_w, target_h) = self.windows[&id].size().await;

        // Calculate where cursor is relative to the target window
        let rel_x = x - target_x;
        let rel_y = y - target_y;

        // Calculate the split side
        let side = if rel_y < target_h / 5.0 {
            SplitSide::Top
        } else if rel_y > target_h * 4.0 / 5.0 {
            SplitSide::Bottom
        } else {
            if rel_x < target_w / 2.0 {
                SplitSide::Left
            } else {
                SplitSide::Right
            }
        };

        self.root.remove(dragged);
        self.root.split_append(id, dragged, side);
    }

    pub async fn dispatch_mouse_move(&mut self, uuid: uuid::Uuid, x: f64, y: f64) {}
}

#[component]
pub fn SubWindowMgr(mut state: Signal<SubWindowMgrState>) -> Element {
    let mut event_queue = use_signal(|| Vec::new());
    let mut dispatch_event = use_future(move || async move {
        let mut events = event_queue.write();
        let mut state = state.write();

        for event in events.drain(..) {
            match event {
                // Events for window dragging and re-positioning functionality
                SubWindowEvent::MouseDown(uuid) => {
                    state.dispatch_mouse_down(uuid);
                }
                SubWindowEvent::MouseUp(uuid, x, y) => {
                    state.dispatch_mouse_up(uuid, x, y).await;
                }
                SubWindowEvent::MouseMove(uuid, x, y) => {
                    state.dispatch_mouse_move(uuid, x, y).await;
                }

                // Events for basic window management functionality
                SubWindowEvent::Close(uuid) => {
                    state.remove(uuid);
                }
                SubWindowEvent::Focus(uuid) => {
                    state.focused = uuid;
                }
            }
        }
    });

    if !event_queue.read().is_empty() {
        // Do not write to the state if there are no events
        // It will cause infinite write ~ read loop
        if dispatch_event.finished() {
            dispatch_event.restart();
        }

        return None;
    }

    rsx! {
        div {
            StylePrelude {}
            { state.read().render(event_queue) }
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum SplitSide {
    Left,
    Right,
    Top,
    Bottom,
}

#[derive(Debug)]
enum VSplitItem {
    Widget(uuid::Uuid),
    HSplit(HSplit),
}

#[derive(Debug)]
struct VSplit {
    children: Vec<VSplitItem>,
}

impl VSplit {
    fn new() -> Self {
        Self {
            children: Vec::new(),
        }
    }

    fn first(&self) -> Option<uuid::Uuid> {
        match self.children.first() {
            Some(VSplitItem::Widget(uuid)) => Some(*uuid),
            Some(VSplitItem::HSplit(hsplit)) => hsplit.first(),
            None => None,
        }
    }

    fn remove(&mut self, id: uuid::Uuid) -> bool {
        self.children.retain_mut(|item| match item {
            VSplitItem::Widget(uuid) => *uuid != id,
            // Remove if the split is empty
            VSplitItem::HSplit(hsplit) => !hsplit.remove(id),
        });

        self.children.is_empty()
    }

    fn append(&mut self, id: uuid::Uuid) {
        self.children.push(VSplitItem::Widget(id));
    }

    fn split_append(&mut self, target: uuid::Uuid, id: uuid::Uuid, side: SplitSide) {
        for (idx, item) in self.children.iter_mut().enumerate() {
            match item {
                VSplitItem::Widget(uuid) => {
                    if *uuid != target {
                        continue;
                    }

                    self.children.insert(idx, VSplitItem::Widget(id));
                    return;
                }
                VSplitItem::HSplit(hsplit) => {
                    hsplit.split_append(target, id, side);
                }
            }
        }
    }

    fn render_element(
        &self,
        nodes: &HashMap<uuid::Uuid, SubWindow>,
        signal: Signal<Vec<SubWindowEvent>>,
    ) -> Element {
        if self.children.is_empty() {
            return None;
        }

        if self.children.len() == 1 {
            return match &self.children[0] {
                VSplitItem::Widget(uuid) => nodes[uuid].render(signal),
                VSplitItem::HSplit(hsplit) => hsplit.render_element(nodes, signal),
            };
        }

        rsx! {
            div { class: "pane panes",
                for item in self.children.iter() {
                    div {
                        match item {
                            VSplitItem::Widget(uuid) => nodes[uuid].render(signal),
                            VSplitItem::HSplit(hsplit) => hsplit.render_element(nodes, signal),
                        },

                        div {
                            class: "splitter-v",
                            onmousedown: |e| {
                                let a = 1;
                            },
                            onmouseup: |e| {
                                let b = 1;
                            },
                            onmousemove: |e| {
                                let c = 1;
                            }
                        }
                    }
                }
            }
        }
    }
}

#[derive(Debug)]
enum HSplitItem {
    Widget(uuid::Uuid),
    VSplit(VSplit),
}

#[derive(Debug)]
struct HSplit {
    children: Vec<HSplitItem>,
}

impl HSplit {
    fn new() -> Self {
        Self {
            children: Vec::new(),
        }
    }

    fn first(&self) -> Option<uuid::Uuid> {
        match self.children.first() {
            Some(HSplitItem::Widget(uuid)) => Some(*uuid),
            Some(HSplitItem::VSplit(vsplit)) => vsplit.first(),
            None => None,
        }
    }

    fn remove(&mut self, id: uuid::Uuid) -> bool {
        self.children.retain_mut(|item| match item {
            HSplitItem::Widget(uuid) => *uuid != id,
            HSplitItem::VSplit(vsplit) => !vsplit.remove(id),
        });

        self.children.is_empty()
    }

    fn split_append(&mut self, target: uuid::Uuid, id: uuid::Uuid, side: SplitSide) {
        for (idx, item) in self.children.iter_mut().enumerate() {
            match item {
                HSplitItem::Widget(uuid) => {
                    if *uuid != target {
                        continue;
                    }

                    self.children.insert(idx, HSplitItem::Widget(id));
                    return;
                }
                HSplitItem::VSplit(vsplit) => {
                    vsplit.split_append(target, id, side);
                }
            }
        }
    }

    fn render_element(
        &self,
        nodes: &HashMap<uuid::Uuid, SubWindow>,
        signal: Signal<Vec<SubWindowEvent>>,
    ) -> Element {
        if self.children.is_empty() {
            return None;
        }

        if self.children.len() == 1 {
            return match &self.children[0] {
                HSplitItem::Widget(uuid) => nodes[uuid].render(signal),
                HSplitItem::VSplit(vsplit) => vsplit.render_element(nodes, signal),
            };
        }

        rsx! {
            div { class: "pane panes",
                for item in self.children.iter() {
                    div {
                        match item {
                            HSplitItem::Widget(uuid) => nodes[uuid].render(signal),
                            HSplitItem::VSplit(vsplit) => vsplit.render_element(nodes, signal),
                        },
                        div { class: "splitter-h" }
                    }
                }
            }
        }
    }
}
