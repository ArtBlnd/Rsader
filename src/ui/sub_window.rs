use std::collections::HashMap;

use crate::ui::widgets::{BoxedWidget, WidgetElement};

use async_channel::{Receiver, Sender};
use dioxus::prelude::*;

use super::utils::MountedDataStorge;

pub struct SubWindow {
    uuid: uuid::Uuid,
    mount_data: MountedDataStorge,

    widget: BoxedWidget,
}

impl SubWindow {
    pub fn new(widget: BoxedWidget) -> Self {
        Self {
            uuid: uuid::Uuid::new_v4(),
            mount_data: MountedDataStorge::new(),

            widget,
        }
    }
}

impl SubWindow {
    fn render(&self) -> Element {
        let uuid = self.uuid;
        let name = self.widget.name();

        let mount_data = self.mount_data.clone();
        rsx! {
            div {
                class: "color-2 pane",
                onmousedown: move |_| SubWindowMgrState::send(SubWindowEvent::Focus(uuid)),
                onmounted: move |data| { mount_data.set(data.data()) },

                SubwindowBar { name, uuid }
                WidgetElement { widget: self.widget.clone() }
            }
        }
    }

    pub async fn position(&self) -> (f64, f64) {
        let mount_data = self.mount_data.get();
        let rect = mount_data.get_client_rect().await.unwrap();

        (rect.origin.x, rect.origin.y)
    }

    pub async fn size(&self) -> (f64, f64) {
        let mount_data = self.mount_data.get();
        let rect = mount_data.get_client_rect().await.unwrap();

        (rect.size.width, rect.size.height)
    }
}

#[component]
fn SubwindowBar(name: String, uuid: uuid::Uuid) -> Element {
    rsx! {
        div { onmousedown: move |_| SubWindowMgrState::send(SubWindowEvent::DragStart(uuid)),

            div { class: "color-2 font-color-w", {name} }
            div {
                class: "font-color-w",
                onmouseup: move |e| {
                    e.stop_propagation();
                    SubWindowMgrState::send(SubWindowEvent::Close(uuid));
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
    height: 100%;
    width: 100%;
    position: relative;
    overflow: hidden;
}
.splitter-h {
    margin-left: -2px;
    margin-right: -2px;
    position: absolute;
    z-index: 3;
    background: #000;
    cursor: ew-resize;
    top: 0;
    bottom: 0;
    width: 2px;
}
.splitter-v {
    margin-top: -2px;
    margin-bottom: -2px;
    cursor: ns-resize;
    left: 0;
    height: 2px;
    position: absolute;
    right: 0;
    z-index: 3;
    background: #000;
}"#;

    rsx! {
        style { {text} }
    }
}

pub enum SubWindowEvent {
    DragStart(uuid::Uuid),
    ResizeStart(uuid::Uuid, f64, f64),
    OnMouseMove(f64, f64),
    OnMouseUp(f64, f64),
    Close(uuid::Uuid),
    Focus(uuid::Uuid),
    WindowCreation(BoxedWidget),
}

type RxEvent = Receiver<SubWindowEvent>;
type TxEvent = Sender<SubWindowEvent>;

pub struct SubWindowMgrState {
    windows: HashMap<uuid::Uuid, SubWindow>,
    root: Split, // Tree of splits

    dragging: Option<uuid::Uuid>,
    resizing: Option<(uuid::Uuid, f64, f64)>,
    focused: uuid::Uuid,
    changed: bool,
}

impl SubWindowMgrState {
    pub fn new() -> Self {
        Self {
            windows: HashMap::new(),
            root: Split::new(),

            dragging: None,
            resizing: None,
            focused: uuid::Uuid::nil(),
            changed: true,
        }
    }

    fn append(&mut self, widget: BoxedWidget) {
        let window = SubWindow::new(widget);
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
        self.mark_changed();
    }

    fn remove(&mut self, uuid: uuid::Uuid) {
        self.root.remove(uuid);
        assert!(self.windows.remove(&uuid).is_some());

        // If the focused window is removed, then set the focused window to the first window
        if self.focused == uuid {
            self.focused = self.root.first().unwrap_or(uuid::Uuid::nil());
        }

        if self.dragging == Some(uuid) {
            self.dragging = None;
        }

        if let Some((target, _, _)) = self.resizing {
            if target == uuid {
                self.resizing = None;
            }
        }

        self.mark_changed();
    }

    async fn find_window_at(&self, x: f64, y: f64) -> Option<uuid::Uuid> {
        for (uuid, window) in self.windows.iter() {
            let (wx, wy) = window.position().await;
            let (ww, wh) = window.size().await;

            if x >= wx && x <= wx + ww && y >= wy && y <= wy + wh {
                return Some(*uuid);
            }
        }

        None
    }

    async fn render_inner(&mut self) -> Element {
        self.changed = false;
        self.root.render_element(&self.windows).await
    }

    fn dispatch_drag_start(&mut self, uuid: uuid::Uuid) {
        self.dragging = Some(uuid);
    }

    fn dispatch_resize_start(&mut self, uuid: uuid::Uuid, x: f64, y: f64) {
        self.resizing = Some((uuid, x, y));
    }

    pub async fn dispatch_mouse_move(&mut self, x: f64, y: f64) {
        // subwindow dragging logic
        if let Some(_) = self.dragging {
            let Some(id) = self.find_window_at(x, y).await else {
                return;
            };

            let (target_x, target_y) = self.windows[&id].position().await;
            let (target_w, target_h) = self.windows[&id].size().await;

            // Calculate where cursor is relative to the target window
            let rel_x = x - target_x;
            let rel_y = y - target_y;

            // Calculate the split side
            let side = if rel_y < target_h * 0.3 {
                SplitSide::Top
            } else if rel_y > target_h * 0.7 {
                SplitSide::Bottom
            } else {
                if rel_x < target_w / 2.0 {
                    SplitSide::Left
                } else {
                    SplitSide::Right
                }
            };

            self.mark_changed();
        }

        if let Some((target, start_x, start_y)) = &mut self.resizing {
            let diff_x = x - *start_x;
            let diff_y = y - *start_y;
            *start_x = x;
            *start_y = y;

            self.root.resize(*target, diff_x, diff_y).await;
            self.mark_changed();
        }
    }

    async fn dispatch_mouse_up(&mut self, x: f64, y: f64) {
        // Subwindow dragging logic
        if let Some(id) = self.dragging.take() {
            let Some(target) = self.find_window_at(x, y).await else {
                return;
            };

            if self.windows.get(&id).is_none() || self.windows.get(&target).is_none() {
                return;
            }

            if target == id {
                return;
            }

            let (target_x, target_y) = self.windows[&target].position().await;
            let (target_w, target_h) = self.windows[&target].size().await;

            // Calculate where cursor is relative to the target window
            let rel_x = x - target_x;
            let rel_y = y - target_y;

            // Calculate the split side
            let side = if rel_y < target_h * 0.2 {
                SplitSide::Top
            } else if rel_y > target_h * 0.8 {
                SplitSide::Bottom
            } else {
                if rel_x < target_w / 2.0 {
                    SplitSide::Left
                } else {
                    SplitSide::Right
                }
            };

            assert!(self.root.remove(id));
            self.root.split_append(target, id, side);
            self.root.sanitize_ratio().await;
            self.mark_changed();
        }

        self.resizing = None;
    }

    pub fn mark_changed(&mut self) {
        self.changed = true;
    }

    pub fn is_changed(&mut self) -> bool {
        self.changed
    }

    pub fn pipe_instance() -> &'static (TxEvent, RxEvent) {
        use once_cell::sync::Lazy;
        static PIPE: Lazy<(TxEvent, RxEvent)> = Lazy::new(|| async_channel::unbounded());

        &PIPE
    }

    pub fn send(event: SubWindowEvent) {
        let _ = Self::pipe_instance().0.try_send(event);
    }

    pub fn tx() -> TxEvent {
        Self::pipe_instance().0.clone()
    }

    pub fn rx() -> RxEvent {
        Self::pipe_instance().1.clone()
    }
}

#[component]
pub fn SubWindowMgr() -> Element {
    let mut pre_rendered = use_resource(|| async {
        let mut state = use_signal(|| SubWindowMgrState::new());
        let rx = SubWindowMgrState::rx();

        let mut state = state.write();
        while !state.is_changed() {
            let mut events = vec![rx.recv().await.unwrap()];
            for _ in 0..64 {
                if let Ok(event) = rx.try_recv() {
                    events.push(event);
                } else {
                    break;
                }
            }

            for event in events {
                match event {
                    SubWindowEvent::DragStart(uuid) => {
                        state.dispatch_drag_start(uuid);
                    }
                    SubWindowEvent::ResizeStart(uuid, x, y) => {
                        state.dispatch_resize_start(uuid, x, y);
                    }
                    SubWindowEvent::OnMouseMove(x, y) => {
                        state.dispatch_mouse_move(x, y).await;
                    }
                    SubWindowEvent::OnMouseUp(x, y) => {
                        state.dispatch_mouse_up(x, y).await;
                    }
                    SubWindowEvent::Close(uuid) => {
                        state.remove(uuid);
                    }
                    SubWindowEvent::Focus(uuid) => {
                        state.focused = uuid;
                        state.mark_changed();
                    }
                    SubWindowEvent::WindowCreation(widget) => {
                        state.append(widget);
                    }
                }
            }
        }

        let element = state.render_inner().await;
        wait_for_next_render().await;

        element
    });

    let element = pre_rendered.read().clone();
    if pre_rendered.finished() {
        pre_rendered.restart();
    }

    rsx! {
        div {
            style: "display: flex; flex-direction: column; width: 100%; overflow: hidden;",
            onmousemove: move |e| {
                e.stop_propagation();
                let coords = e.client_coordinates();
                SubWindowMgrState::send(SubWindowEvent::OnMouseMove(coords.x, coords.y));
            },
            onmouseup: move |e| {
                e.stop_propagation();
                let coords = e.client_coordinates();
                SubWindowMgrState::send(SubWindowEvent::OnMouseUp(coords.x, coords.y));
            },

            StylePrelude {}
            { element }
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

impl SplitSide {
    fn rotate(&self) -> Self {
        match self {
            SplitSide::Left => SplitSide::Top,
            SplitSide::Right => SplitSide::Bottom,
            SplitSide::Top => SplitSide::Right,
            SplitSide::Bottom => SplitSide::Left,
        }
    }
}

enum SplitItem {
    Widget(uuid::Uuid),
    Split(Split),
}

impl SplitItem {
    pub fn uuid(&self) -> uuid::Uuid {
        match self {
            SplitItem::Widget(uuid) => *uuid,
            SplitItem::Split(split) => split.uuid,
        }
    }
}

struct Split {
    children: Vec<SplitItem>,
    children_ratio: Vec<f64>,

    uuid: uuid::Uuid,
    rect: MountedDataStorge,
    horizontal: bool,
}

impl Split {
    fn new() -> Self {
        Self {
            children: Vec::new(),
            children_ratio: Vec::new(),

            uuid: uuid::Uuid::new_v4(),
            rect: MountedDataStorge::new(),
            horizontal: false,
        }
    }

    fn new_with(top: uuid::Uuid, bottom: uuid::Uuid, horizontal: bool) -> Self {
        Self {
            children: vec![SplitItem::Widget(top), SplitItem::Widget(bottom)],
            children_ratio: vec![0.5, 0.5],

            uuid: uuid::Uuid::new_v4(),
            rect: MountedDataStorge::new(),
            horizontal,
        }
    }

    fn first(&self) -> Option<uuid::Uuid> {
        match self.children.first() {
            Some(SplitItem::Widget(uuid)) => Some(*uuid),
            Some(SplitItem::Split(split)) => split.first(),
            None => None,
        }
    }

    /// Remove the window with the given id from the split tree
    /// Returns true if the window is removed
    fn remove(&mut self, id: uuid::Uuid) -> bool {
        if let Some(target) = self.find(id) {
            self.remove_and_rebalance(target);
            return true;
        } else {
            for (idx, item) in self.children.iter_mut().enumerate() {
                if let SplitItem::Split(split) = item {
                    if split.remove(id) {
                        if split.children.is_empty() {
                            self.remove_and_rebalance(idx);
                        }

                        return true;
                    }
                }
            }
        }

        false
    }

    /// Remove the window at the given index and rebalance the ratios
    fn remove_and_rebalance(&mut self, idx: usize) {
        let ratio = self.children_ratio.remove(idx);
        self.children.remove(idx);

        if !self.children_ratio.is_empty() {
            self.children_ratio[idx.checked_sub(1).unwrap_or_default()] += ratio;
        }
    }

    fn append(&mut self, id: uuid::Uuid) {
        self.children.push(SplitItem::Widget(id));

        if self.children_ratio.is_empty() {
            self.children_ratio.push(1.0);
        } else {
            let ratio = 1.0 / (self.children_ratio.len() + 1) as f64;
            for r in self.children_ratio.iter_mut() {
                *r -= ratio;
            }
            self.children_ratio.push(ratio);
        }
    }

    fn find(&self, id: uuid::Uuid) -> Option<usize> {
        self.children.iter().position(|item| item.uuid() == id)
    }

    async fn px_per_ratio(&self) -> f64 {
        let rect = self.rect.get().get_client_rect().await.unwrap();
        if self.horizontal {
            1.0 / rect.size.width
        } else {
            1.0 / rect.size.height
        }
    }

    async fn sanitize_ratio(&mut self) {
        // Sanitize the total ratio to be 1.0
        let total = self.children_ratio.iter().sum::<f64>();
        for ratio in self.children_ratio.iter_mut() {
            *ratio /= total;
        }

        // Sanitize the min ratio to be 100px
        let min_amount = 100.0 * self.px_per_ratio().await;
        let mut debt = 0.0;
        for ratio in self.children_ratio.iter_mut() {
            if *ratio < min_amount {
                *ratio = min_amount;
                debt += min_amount - *ratio;
            } else {
                *ratio += debt;
                debt = 0.0;
            }
        }
    }

    #[async_recursion::async_recursion(?Send)]
    async fn resize(&mut self, id: uuid::Uuid, w: f64, h: f64) {
        let amount_px = if self.horizontal { w } else { h };
        let amount = amount_px * self.px_per_ratio().await;
        let min_amount = 100.0 * self.px_per_ratio().await;
        if let Some(target) = self.find(id) {
            let old_ratio0 = self.children_ratio[target];
            let old_ratio1 = self.children_ratio[target - 1];

            self.children_ratio[target] -= amount;
            self.children_ratio[target - 1] += amount;

            let cur_ratio0 = self.children_ratio[target];
            let cur_ratio1 = self.children_ratio[target - 1];

            if cur_ratio0 < min_amount || cur_ratio1 < min_amount {
                self.children_ratio[target] = old_ratio0;
                self.children_ratio[target - 1] = old_ratio1;
            }
        } else {
            for item in self.children.iter_mut() {
                if let SplitItem::Split(split) = item {
                    split.resize(id, w, h).await;
                }
            }
        }
    }

    fn split_append(&mut self, target: uuid::Uuid, id: uuid::Uuid, side: SplitSide) -> bool {
        for (idx, item) in self.children.iter_mut().enumerate() {
            let ratio = self.children_ratio[idx];
            match item {
                SplitItem::Widget(uuid) => {
                    if *uuid != target {
                        continue;
                    }

                    let side = if self.horizontal { side.rotate() } else { side };
                    match side {
                        SplitSide::Top => {
                            self.children.insert(idx, SplitItem::Widget(id));
                            self.children_ratio.insert(idx, ratio / 2.0);
                            self.children_ratio[idx + 1] = ratio / 2.0;
                        }
                        SplitSide::Bottom => {
                            self.children.insert(idx + 1, SplitItem::Widget(id));
                            self.children_ratio.insert(idx + 1, ratio / 2.0);
                            self.children_ratio[idx] = ratio / 2.0;
                        }
                        SplitSide::Left => {
                            let split = Split::new_with(id, *uuid, !self.horizontal);
                            self.children[idx] = SplitItem::Split(split);
                        }
                        SplitSide::Right => {
                            let split = Split::new_with(*uuid, id, !self.horizontal);
                            self.children[idx] = SplitItem::Split(split);
                        }
                    }
                    return true;
                }
                SplitItem::Split(split) => {
                    if split.split_append(target, id, side) {
                        return true;
                    }
                }
            }
        }

        return false;
    }

    #[async_recursion::async_recursion(?Send)]
    async fn render_element(&self, nodes: &HashMap<uuid::Uuid, SubWindow>) -> Element {
        if self.children.is_empty() {
            return None;
        }

        let divider_class = if self.horizontal {
            "splitter-h"
        } else {
            "splitter-v"
        };

        let mut rendered_elements = Vec::new();
        for (idx, item) in self.children.iter().enumerate() {
            let uuid = item.uuid();
            let inner = match item {
                SplitItem::Widget(uuid) => nodes[uuid].render(),
                SplitItem::Split(split) => split.render_element(nodes).await,
            };

            rendered_elements.push(rsx! {
                div { key: "{item.uuid()}",

                    // divider
                    if idx != 0 {
                        div {
                            class: "{divider_class}",
                            onmousedown: move |e| {
                                e.stop_propagation();
                                let coords = e.client_coordinates();
                                SubWindowMgrState::send(SubWindowEvent::ResizeStart(uuid, coords.x, coords.y));
                            }
                        }
                    }

                    // sub window itself
                    { inner }
                }
            })
        }

        let grid_templete_ratio = self
            .children_ratio
            .iter()
            .map(|ratio| format!("{}fr ", ratio))
            .collect::<String>();

        let style = if self.horizontal {
            format!("grid-auto-flow: column; grid-template-columns: {grid_templete_ratio}")
        } else {
            format!("grid-auto-flow: row; grid-template-rows: {grid_templete_ratio}")
        };

        let rect = self.rect.clone();
        rsx! {
            div {
                class: "pane panes",
                style: "{style}",
                onmounted: move |data| { rect.set(data.data()) },
                for element in rendered_elements.iter() {
                    { element }
                }
            }
        }
    }
}
