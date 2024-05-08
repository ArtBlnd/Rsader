use iced::{
    alignment::{Horizontal, Vertical},
    mouse::Cursor,
    widget::canvas::{Event, Frame, Geometry, Path, Program, Text},
    Color, Font, Pixels, Point, Rectangle, Renderer, Size, Theme,
};
use iced_core::{event, font, mouse};
use num_traits::ToPrimitive;
use rust_decimal::Decimal;

use crate::{exchange::Orderbook, window::utils};

pub struct OrderbookProgram {
    orderbook: Orderbook,
}

impl OrderbookProgram {
    pub fn new(orderbook: Orderbook) -> Self {
        Self {
            orderbook: orderbook.normalize(),
        }
    }
}

impl<Message> Program<Message> for OrderbookProgram {
    type State = ();

    fn update(
        &self,
        _state: &mut Self::State,
        _event: Event,
        _bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> (event::Status, Option<Message>) {
        (event::Status::Captured, None)
    }

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: Cursor,
    ) -> Vec<Geometry> {
        let max_amount = self.orderbook.max_amount();
        let max_price = self.orderbook.max_price();
        let len = self.orderbook.bids.len() + self.orderbook.asks.len();
        let mut text_font = Font::with_name("Space Mono");
        text_font.weight = font::Weight::Normal;

        let width = bounds.width;
        let height = bounds.height;
        let size = Size::new(width, height);

        let height_item = (height / len as f32).min(35.0).max(25.0);

        let text = Text {
            content: format!("{max_amount}"),
            size: Pixels(height_item * 0.5),
            position: Point::new(0.0, 0.0),
            font: text_font,
            ..Default::default()
        };
        let (maw, mah) = utils::estimate_text_rect(&text);
        let text = Text {
            content: format!("{max_price}"),
            size: Pixels(height_item * 0.5),
            position: Point::new(0.0, 0.0),
            font: text_font,
            ..Default::default()
        };
        let mpw = utils::estimate_text_rect(&text).0;

        let min_width = mpw + maw + 50.0;
        let scale = (width / min_width).min(1.0);

        let bar_color_ask = Color::from_rgb8(54, 27, 34);
        let bar_color_bid = Color::from_rgb8(21, 47, 30);
        let bar_width = |amount: Decimal| (amount / max_amount).to_f32().unwrap() * width;
        let bar_margin = 2.0;
        let font_color1 = Color::from_rgb8(202, 63, 100);
        let font_color2 = Color::from_rgb8(37, 166, 79);
        let font_color3 = Color::from_rgb8(255, 255, 255);
        let font_w_margin = 10.0;
        let font_h_margin = height_item - mah;
        let font_size = Pixels(height_item * 0.5 * scale);

        let mut frmae_background = Frame::new(renderer, size);
        let mut frame_bar = Frame::new(renderer, size);
        let mut frame_text = Frame::new(renderer, size);

        frmae_background.fill(
            &Path::rectangle(Point::ORIGIN, Size { width, height }),
            Color::from_rgb8(18, 18, 18),
        );

        let mut y = (height / 2.0) - (self.orderbook.asks.len() as f32 * height_item);
        for ask in self.orderbook.asks.iter().rev() {
            let bar_width = bar_width(ask.amount);
            let bar = Path::rectangle(
                Point::new(width - bar_width, y + bar_margin),
                Size::new(bar_width, height_item - bar_margin * 2.0),
            );
            frame_bar.fill(&bar, bar_color_ask);

            Text {
                content: format!("{}", ask.price),
                size: font_size,
                position: Point::new(font_w_margin, y + font_h_margin),
                font: text_font,
                color: font_color1,
                vertical_alignment: Vertical::Center,
                ..Default::default()
            }
            .draw_with(|path, fill| {
                frame_text.fill(&path, fill);
            });

            Text {
                content: format!("{}", ask.amount),
                size: font_size,
                position: Point::new(width - maw * scale - font_w_margin * 2.0, y + font_h_margin),
                font: text_font,
                color: font_color3,
                horizontal_alignment: Horizontal::Left,
                vertical_alignment: Vertical::Center,
                ..Default::default()
            }
            .draw_with(|path, fill| {
                frame_text.fill(&path, fill);
            });

            y += height_item;
        }

        for bid in self.orderbook.bids.iter() {
            let bar_width = bar_width(bid.amount);
            let bar = Path::rectangle(
                Point::new(width - bar_width, y + bar_margin),
                Size::new(bar_width, height_item - bar_margin * 2.0),
            );
            frame_bar.fill(&bar, bar_color_bid);

            Text {
                content: format!("{}", bid.price),
                size: font_size,
                position: Point::new(font_w_margin, y + font_h_margin),
                font: text_font,
                color: font_color2,
                vertical_alignment: Vertical::Center,
                ..Default::default()
            }
            .draw_with(|path, fill| {
                frame_text.fill(&path, fill);
            });

            Text {
                content: format!("{}", bid.amount),
                size: font_size,
                position: Point::new(width - maw * scale - font_w_margin * 2.0, y + font_h_margin),
                font: text_font,
                color: font_color3,
                horizontal_alignment: Horizontal::Left,
                vertical_alignment: Vertical::Center,
                ..Default::default()
            }
            .draw_with(|path, fill| {
                frame_text.fill(&path, fill);
            });

            y += height_item;
        }

        vec![
            frmae_background.into_geometry(),
            frame_bar.into_geometry(),
            frame_text.into_geometry(),
        ]
    }
}
