use std::collections::HashMap;

use iced::{
    keyboard::{key::Named, Key, Modifiers},
    widget::{
        pane_grid::{self, Pane},
        row,
    },
    Border, Color, Length, Shadow,
};

use super::{
    action_list, candle_chart,
    commander::{self, Commander},
    console, orderbook, settings,
    sub_window::SubWindow,
    widgets::{Keybind, Shortcut},
};
use crate::{broadcast, global_context::GlobalContext};

pub struct Dashboard {
    global_ctx: GlobalContext,

    settings: settings::Settings,

    commander: Commander,
    is_modal_open: bool,

    sw_consoles: HashMap<uuid::Uuid, SubWindow<console::Console>>,
    sw_orderbooks: HashMap<uuid::Uuid, SubWindow<orderbook::OrderbookWindow>>,
    sw_actions: HashMap<uuid::Uuid, SubWindow<action_list::ActionListWindow>>,
    sw_candle_charts: HashMap<uuid::Uuid, SubWindow<candle_chart::CandleChartWindow>>,

    sidebar_selected: String,
    sidebar_items: HashMap<String, pane_grid::State<(SubwindowType, uuid::Uuid)>>,
}

impl Dashboard {
    pub fn new(global_ctx: &GlobalContext) -> Self {
        let main_console = console::Console::new(global_ctx);
        let main_console_uuid = uuid::Uuid::new_v4();

        let (main, _) = pane_grid::State::new((SubwindowType::Console, main_console_uuid));

        Self {
            global_ctx: global_ctx.clone(),
            settings: settings::Settings::new(),

            commander: Commander::new(),
            is_modal_open: false,

            sw_consoles: [(
                main_console_uuid,
                SubWindow::new(main_console_uuid, main_console),
            )]
            .try_into()
            .unwrap(),
            sw_orderbooks: HashMap::new(),
            sw_actions: HashMap::new(),
            sw_candle_charts: HashMap::new(),

            sidebar_selected: "Main".to_string(),
            sidebar_items: [("Main".to_string(), main)].try_into().unwrap(),
        }
    }

    pub fn refresh_now(&mut self) {
        for (_, console) in self.sw_consoles.iter_mut() {
            console.refresh_now();
        }
        for (_, orderbook) in self.sw_orderbooks.iter_mut() {
            orderbook.refresh_now();
        }
        for (_, actions) in self.sw_actions.iter_mut() {
            actions.refresh_now();
        }
    }

    pub fn run_command(&mut self, command: String) -> Option<()> {
        let command = command.trim().to_uppercase();
        let parts: Vec<&str> = command.split_ascii_whitespace().collect();
        match parts.as_slice() {
            ["ORDERBOOK" | "OB" | "BOOK", exchange, pair] => {
                let (base, quote) = pair.split_once("-")?;
                let (base, quote) = (base.parse().ok()?, quote.parse().ok()?);
                let orderbook_window = orderbook::OrderbookWindow::new(
                    (base, quote),
                    &exchange.to_lowercase(),
                    &self.global_ctx,
                );
                let orderbook_window_uuid = uuid::Uuid::new_v4();

                let grid = self.sidebar_items.get_mut(&self.sidebar_selected).unwrap();
                let (pane, _) = grid.panes.iter().next().unwrap();
                grid.split(
                    pane_grid::Axis::Vertical,
                    pane.clone(),
                    (SubwindowType::Orderbook, orderbook_window_uuid),
                );
                self.sw_orderbooks.insert(
                    orderbook_window_uuid,
                    SubWindow::new(orderbook_window_uuid, orderbook_window),
                );
            }
            ["CHART", exchange, pair] => {
                let (base, quote) = pair.split_once("-")?;
                let (base, quote) = (base.parse().ok()?, quote.parse().ok()?);
                let candle_chart_window = candle_chart::CandleChartWindow::new(
                    (base, quote),
                    &exchange.to_lowercase(),
                    &self.global_ctx,
                );
                let candle_chart_window_uuid = uuid::Uuid::new_v4();

                let grid = self.sidebar_items.get_mut(&self.sidebar_selected).unwrap();
                let (pane, _) = grid.panes.iter().next().unwrap();
                grid.split(
                    pane_grid::Axis::Vertical,
                    pane.clone(),
                    (SubwindowType::CandleChart, candle_chart_window_uuid),
                );

                self.sw_candle_charts.insert(
                    candle_chart_window_uuid,
                    SubWindow::new(candle_chart_window_uuid, candle_chart_window),
                );
            }
            ["ACTIONS"] => {
                let actions_window = action_list::ActionListWindow::new(&self.global_ctx);
                let actions_window_uuid = uuid::Uuid::new_v4();

                let grid = self.sidebar_items.get_mut(&self.sidebar_selected).unwrap();
                let (pane, _) = grid.panes.iter().next().unwrap();
                grid.split(
                    pane_grid::Axis::Vertical,
                    pane.clone(),
                    (SubwindowType::Actions, actions_window_uuid),
                );
                self.sw_actions.insert(
                    actions_window_uuid,
                    SubWindow::new(actions_window_uuid, actions_window),
                );
            }
            ["CONSOLE"] => {
                let console = console::Console::new(&self.global_ctx);
                let console_uuid = uuid::Uuid::new_v4();

                let grid = self.sidebar_items.get_mut(&self.sidebar_selected).unwrap();
                let (pane, _) = grid.panes.iter().next().unwrap();
                grid.split(
                    pane_grid::Axis::Vertical,
                    pane.clone(),
                    (SubwindowType::Console, console_uuid),
                );
                self.sw_consoles
                    .insert(console_uuid, SubWindow::new(console_uuid, console));
            }
            ["BALANCES", exchange] => {
                todo!()
            }
            _ => {}
        }

        None
    }

    pub fn update(&mut self, message: Message) -> iced::Command<Message> {
        match message {
            Message::CloseModal => {
                self.is_modal_open = false;
            }
            Message::OpenModal => {
                self.is_modal_open = true;
                return iced::widget::text_input::focus(self.commander.id().clone());
            }
            Message::ConsoleMessage(message, uuid) => {
                self.sw_consoles.get_mut(&uuid).unwrap().update(message);
            }
            Message::OrderbookMessage(message, uuid) => {
                self.sw_orderbooks.get_mut(&uuid).unwrap().update(message);
            }
            Message::ActionsMessage(message, uuid) => {
                self.sw_actions.get_mut(&uuid).unwrap().update(message);
            }
            Message::CommanderMessage(message) => {
                if let Some(command) = self.commander.update(message) {
                    self.run_command(command);
                    self.is_modal_open = false;
                }
            }
            Message::CandleChartMessage(message, uuid) => {
                self.sw_candle_charts
                    .get_mut(&uuid)
                    .unwrap()
                    .update(message);
            }
            Message::AddOrderbook => {}
            Message::OnSidebarButtonPressed { name } => {
                self.sidebar_selected = name;
            }
            Message::Focused(_) => {}
            Message::Resized(resize_event) => {
                let window = self.sidebar_items.get_mut(&self.sidebar_selected).unwrap();
                window.resize(resize_event.split, resize_event.ratio);
            }
            Message::Dragged(pane_grid::DragEvent::Dropped { pane, target }) => {
                let window = self.sidebar_items.get_mut(&self.sidebar_selected).unwrap();
                window.drop(pane, target);
            }
            Message::Dragged(_) => {}
        }

        return iced::Command::none();
    }

    pub fn broadcast(&mut self, item: broadcast::Item) {
        for (_, console) in self.sw_consoles.iter_mut() {
            console.broadcast(item.clone());
        }

        for (_, orderbook) in self.sw_orderbooks.iter_mut() {
            orderbook.broadcast(item.clone());
        }

        for (_, actions) in self.sw_actions.iter_mut() {
            actions.broadcast(item.clone());
        }

        for (_, candle_chart) in self.sw_candle_charts.iter_mut() {
            candle_chart.broadcast(item.clone());
        }
    }

    pub fn view(&self) -> iced::Element<'_, Message> {
        let mut sidebar = iced::widget::column!().spacing(1).padding(10).width(150);
        for (name, _) in self.sidebar_items.iter() {
            let button = iced::widget::Button::new(iced::widget::Text::new(name))
                .padding(10)
                .width(iced::Length::Fill)
                .on_press(Message::OnSidebarButtonPressed { name: name.clone() });
            sidebar = sidebar.push(button);
        }

        let pane_grid_state = &self.sidebar_items[&self.sidebar_selected];
        let pane_grid = pane_grid::PaneGrid::new(&pane_grid_state, move |_, &(ty, idx), _| {
            let (content, title) = match ty {
                SubwindowType::Console => {
                    let console = &self.sw_consoles[&idx];
                    (
                        console
                            .view()
                            .map(move |msg| Message::ConsoleMessage(msg, idx)),
                        console.title(),
                    )
                }
                SubwindowType::Orderbook => {
                    let orderbook = &self.sw_orderbooks[&idx];
                    (
                        orderbook
                            .view()
                            .map(move |msg| Message::OrderbookMessage(msg, idx)),
                        orderbook.title(),
                    )
                }
                SubwindowType::Actions => {
                    let actions = &self.sw_actions[&idx];
                    (
                        actions
                            .view()
                            .map(move |msg| Message::ActionsMessage(msg, idx)),
                        actions.title(),
                    )
                }
                SubwindowType::CandleChart => {
                    let candle_chart = &self.sw_candle_charts[&idx];
                    (
                        candle_chart
                            .view()
                            .map(move |msg| Message::CandleChartMessage(msg, idx)),
                        candle_chart.title(),
                    )
                }
            };

            let title_bar = pane_grid::TitleBar::new(
                iced::widget::Text::new(format!(" {}", &title))
                    .vertical_alignment(iced::alignment::Vertical::Center)
                    .width(iced::Length::Fill)
                    .height(25),
            )
            .padding(10)
            .style(iced::widget::container::Appearance {
                text_color: Some(Color::from_rgb8(74, 149, 159)),
                background: Some(iced::Background::Color(Color::from_rgb8(00, 00, 00))),
                shadow: Shadow::default(),
                border: Border::default(),
            });

            pane_grid::Content::new(content).title_bar(title_bar)
        })
        .spacing(5)
        .on_resize(10, Message::Resized)
        .on_drag(Message::Dragged)
        .on_click(Message::Focused);

        let keybind = Shortcut::from_keybind(
            iced::widget::space::Space::new(Length::Shrink, Length::Shrink),
            Keybind::new(Key::Named(Named::Space), Modifiers::CTRL),
            || Message::OpenModal,
        );

        iced_aw::native::Modal::new(
            row![keybind, sidebar, pane_grid],
            self.is_modal_open
                .then(|| self.commander.view().map(Message::CommanderMessage)),
        )
        .align_y(iced::alignment::Vertical::Top)
        .backdrop(Message::CloseModal)
        .on_esc(Message::CloseModal)
        .into()
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    CloseModal,
    OpenModal,
    AddOrderbook,
    Resized(pane_grid::ResizeEvent),
    Dragged(pane_grid::DragEvent),
    Focused(Pane),
    CommanderMessage(commander::Message),
    OnSidebarButtonPressed { name: String },
    ActionsMessage(action_list::Message, uuid::Uuid),
    OrderbookMessage(orderbook::Message, uuid::Uuid),
    ConsoleMessage(console::Message, uuid::Uuid),
    CandleChartMessage(candle_chart::Message, uuid::Uuid),
}

#[derive(Debug, Clone, Copy)]
pub enum SubwindowType {
    Console,
    Orderbook,
    Actions,
    CandleChart,
}
