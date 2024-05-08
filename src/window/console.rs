use std::{
    borrow::Cow,
    cell::RefCell,
    pin::Pin,
    task::{Context, Poll},
};

use crossbeam::channel::{Receiver, Sender};
use futures::Future;
use iced::widget::{column, scrollable::Viewport};

use crate::{
    broadcast,
    global_context::GlobalContext,
    vm::{
        action::{ActionExecutor, EvaluateAction},
        context::VmContext,
    },
};

use super::sub_window::SubWindowContent;

pub struct Console {
    buffer: String,
    text: Vec<String>,

    pipe_rx: Receiver<String>,
    pipe_tx: Sender<String>,

    global_context: GlobalContext,
    vm_context: VmContext,
}

impl Console {
    pub fn new(glob_ctx: &GlobalContext) -> Self {
        let (pipe_tx, pipe_rx) = crossbeam::channel::unbounded();

        Self {
            buffer: String::new(),
            text: Vec::new(),

            pipe_rx,
            pipe_tx,

            global_context: glob_ctx.clone(),
            vm_context: glob_ctx.new_vm_context(),
        }
    }

    fn new_pipe(&self) -> ConsolePipe {
        ConsolePipe {
            global_ctx: self.global_context.clone(),
            pipe: self.pipe_tx.clone(),
        }
    }

    fn recv_all(&mut self) {
        while let Ok(message) = self.pipe_rx.try_recv() {
            self.text.push(message);
        }
    }

    fn execute_action(&mut self, code: String) {
        let at = ActionExecutor::new(EvaluateAction {
            input: code,
            context: self.vm_context.new_scope(vec![]),
        })
        .spawn_rt(&self.global_context, self.new_pipe());
        self.global_context.insert_action(at);
    }
}

impl SubWindowContent for Console {
    type Message = Message;

    fn title(&self) -> &str {
        "Console"
    }

    fn view(&self) -> iced::Element<'_, Self::Message> {
        let commands = iced::widget::Scrollable::new(column(self.text.iter().map(|v| {
            use iced::widget::Text;
            Text::new(Cow::Borrowed(v.as_str())).into()
        })))
        .direction(iced::widget::scrollable::Direction::Vertical(
            iced::widget::scrollable::Properties::default(),
        ))
        .width(iced::Length::Fill)
        .height(iced::Length::Fill)
        .on_scroll(Message::OnScrolled);

        let command_bar = iced::widget::text_input("help for commands...", &self.buffer)
            .on_submit(Message::OnEnter)
            .on_input(Message::OnInput);

        column![commands, command_bar]
            .align_items(iced::Alignment::End)
            .padding(10)
            .into()
    }

    fn refresh_now(&mut self) {
        self.recv_all();
    }

    fn update(&mut self, message: Self::Message) {
        self.recv_all();
        match message {
            Message::None => {}
            Message::OnScrolled(_) => {}
            Message::OnEnter => {
                let buffer = std::mem::take(&mut self.buffer);
                self.text.push(buffer.clone());
                self.execute_action(buffer);
            }
            Message::OnInput(input) => {
                self.buffer = input;
            }
        }
    }

    fn broadcast(&mut self, _item: broadcast::Item) {}
}

#[derive(Debug, Clone)]
pub enum Message {
    None,
    OnScrolled(Viewport),
    OnEnter,
    OnInput(String),
}

#[derive(Clone)]
pub struct ConsolePipe {
    global_ctx: GlobalContext,
    pipe: Sender<String>,
}

thread_local! {
    static CONSOLE: RefCell<Option<ConsolePipe>> = RefCell::new(None);
}

impl ConsolePipe {
    pub fn log(&self, message: String) {
        self.pipe.send(message).unwrap();
        self.global_ctx.refresh_now();
    }

    pub fn instance() -> Self {
        CONSOLE.with(|console| console.borrow().as_ref().unwrap().clone())
    }

    pub fn enter(self) {
        CONSOLE.with(|console| {
            *console.borrow_mut() = Some(self);
        });
    }

    pub fn leave() -> Self {
        CONSOLE.with(|console| console.borrow_mut().take()).unwrap()
    }
}

#[macro_export]
macro_rules! console {
    ($($arg:tt)*) => {
        crate::window::console::ConsolePipe::instance().log(
            format!($($arg)*)
        );
    };
}

#[pin_project::pin_project]
pub struct ConsoleInstrument<T> {
    #[pin]
    inner: T,
    console_pipe: Option<ConsolePipe>,
}

impl<T> ConsoleInstrument<T> {
    pub fn new(inner: T, console_pipe: ConsolePipe) -> Self {
        Self {
            inner,
            console_pipe: Some(console_pipe),
        }
    }
}

impl<T> Future for ConsoleInstrument<T>
where
    T: Future,
{
    type Output = T::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        this.console_pipe.take().unwrap().enter();
        let result = this.inner.poll(cx);
        *this.console_pipe = Some(ConsolePipe::leave());
        result
    }
}
