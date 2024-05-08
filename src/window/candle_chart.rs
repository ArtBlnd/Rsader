use std::{ops::Range, time::Duration};

use iced_core::mouse;
use plotters::{
    coord::ranged1d::{DefaultFormatting, KeyPointHint},
    prelude::*,
};
use plotters_iced::{Chart, ChartBuilder, DrawingBackend};
use rust_decimal::prelude::*;
use rust_decimal_macros::dec;

use crate::{
    broadcast::{self, BroadcastFrom},
    currency::Currency,
    exchange::{CandleSticks, Exchange},
    global_context::GlobalContext,
    utils::async_helpers,
};

use super::sub_window::SubWindowContent;

pub struct CandleChartWindow {
    title: String,
    pair: (Currency, Currency),
    exchange_name: String,
    global_ctx: GlobalContext,

    chart: Option<CandleChart>,
}

impl CandleChartWindow {
    pub fn new(
        pair: (Currency, Currency),
        exchange_name: &str,
        global_ctx: &GlobalContext,
    ) -> Self {
        global_ctx.ex().bithumb.subscribe(pair, None);
        {
            let ctx = global_ctx.clone();
            global_ctx.spawn(async move {
                loop {
                    let candle_stick = ctx.ex().bithumb.candlesticks(pair, None).await.unwrap();
                    ctx.broadcaster()
                        .broadcast(Some(BroadcastFrom::Exchange("bithumb")), candle_stick);
                    async_helpers::sleep(Duration::from_secs(1)).await;
                }
            });
        }

        Self {
            title: format!("CandleChart({}): {}-{}", exchange_name, pair.0, pair.1),
            pair,
            exchange_name: exchange_name.to_string(),
            global_ctx: global_ctx.clone(),
            chart: None,
        }
    }
}

impl SubWindowContent for CandleChartWindow {
    type Message = Message;

    fn title(&self) -> &str {
        &self.title
    }

    fn view(&self) -> iced::Element<'_, Self::Message> {
        if let Some(chart) = &self.chart {
            plotters_iced::ChartWidget::new(chart).into()
        } else {
            iced::widget::Text::new("No candle chart data").into()
        }
    }

    fn refresh_now(&mut self) {}
    fn update(&mut self, _message: Self::Message) {}

    fn broadcast(&mut self, item: broadcast::Item) {
        let Some(BroadcastFrom::Exchange(from)) = item.from() else {
            return;
        };

        if from != &self.exchange_name {
            return;
        }

        let Some(candle_stick) = item.as_ref::<CandleSticks>() else {
            return;
        };

        if candle_stick.pair != self.pair {
            return;
        }

        self.chart = Some(CandleChart {
            candle_sticks: candle_stick.clone(),
            interval: 60,
        });
    }
}

#[derive(Debug, Clone)]
pub struct Message {}

#[derive(Debug, Clone)]
struct CandleChart {
    candle_sticks: CandleSticks,
    interval: u64,
}

#[derive(Debug, Default)]
pub struct CandleChartState {
    mouse_position: Option<(f32, f32)>,
}

impl Chart<Message> for CandleChart {
    type State = CandleChartState;

    fn draw_chart<DB: DrawingBackend>(
        &self,
        state: &Self::State,
        root: DrawingArea<DB, plotters::coord::Shift>,
    ) {
        let (width, height) = root.dim_in_pixel();
        let mut builder = ChartBuilder::on(&root);

        let scale = &self.candle_sticks.tickers[0].high.scale();

        let data = self
            .candle_sticks
            .tickers
            .iter()
            .skip(self.candle_sticks.tickers.len() - 100);

        let high = data
            .clone()
            .map(|v| v.high)
            .max()
            .unwrap_or_default()
            .to_f64()
            .unwrap_or_default();

        let low = data
            .clone()
            .map(|v| v.low)
            .min()
            .unwrap_or_default()
            .to_f64()
            .unwrap_or_default();

        let timestamps = data.clone().map(|v| v.timestamp);
        let timestamp_min = timestamps.clone().min().unwrap_or_default();
        let timestamp_max = timestamps.max().unwrap_or_default();

        let timestamp_min = chrono::DateTime::from_timestamp_millis(timestamp_min as i64).unwrap();
        let timestamp_max = chrono::DateTime::from_timestamp_millis(timestamp_max as i64).unwrap();

        // let (label_w, _) = utils::estimate_text_rect2(&high.to_string(), "sans-serif", 12.0);
        // let (_, label_h) =
        //     utils::estimate_text_rect2(&timestamp_min.to_string(), "sans-serif", 12.0);

        let mut chart = builder
            .x_label_area_size(40)
            .y_label_area_size(40)
            .margin(20)
            .build_cartesian_2d(
                RangedDateTime::from(timestamp_min..timestamp_max),
                low..high,
            )
            .unwrap();

        chart
            .configure_mesh()
            .x_label_style(&RGBColor(166, 168, 177))
            .y_label_style(&RGBColor(166, 168, 177))
            .axis_style(&RGBColor(121, 125, 130))
            .bold_line_style(&RGBColor(32, 33, 36))
            .draw()
            .unwrap();

        let data_count = data.clone().count() as u32;

        let candles = data.map(|v| {
            let mut close = v.close.clone();
            if v.open == v.close {
                close = v.close * dec!(0.9999);
            }

            CandleStick::new(
                chrono::DateTime::from_timestamp_millis(v.timestamp as i64).unwrap(),
                v.open.to_f64().unwrap(),
                v.high.to_f64().unwrap(),
                v.low.to_f64().unwrap(),
                close.to_f64().unwrap(),
                RGBColor(37, 167, 80).filled(),
                RGBColor(202, 63, 100).filled(),
                (width / data_count).checked_sub(2).unwrap_or(1),
            )
        });

        chart.draw_series(candles.skip(1)).unwrap();

        if let Some((x, y)) = &state.mouse_position {
            let timestamp_span =
                timestamp_max.timestamp_micros() - timestamp_min.timestamp_micros();
            let time_per_pixel = timestamp_span as f64 / (width as f64 - 80.0);

            let height_span = high - low;
            let height_per_pixel = height_span / (height as f64 - 80.0);

            let (x, y) = (
                chrono::DateTime::from_timestamp_micros(
                    timestamp_min.timestamp_micros() + (time_per_pixel * *x as f64) as i64
                        - (time_per_pixel * 60.0) as i64,
                )
                .unwrap(),
                high - height_per_pixel * *y as f64 + height_per_pixel * 20.0,
            );

            let line_style = WHITE.mix(0.2).filled();

            let vertical = plotters::element::PathElement::new([(x, low), (x, high)], line_style);
            let horizontal = plotters::element::PathElement::new(
                [(timestamp_min, y), (timestamp_max, y)],
                line_style,
            );

            chart.draw_series([vertical, horizontal]).unwrap();

            let text: TextStyle = ("sans-serif", 12).into();

            let mut t = Decimal::from_f64(y).unwrap();
            t.rescale(*scale);

            let text = plotters::element::Text::new(
                t.to_string(),
                (timestamp_min, y),
                text.color(&RGBColor(166, 168, 177)),
            );

            chart.draw_series([text]).unwrap();
        }
    }

    fn build_chart<DB>(&self, _state: &Self::State, mut _builder: ChartBuilder<DB>)
    where
        DB: DrawingBackend,
    {
    }

    fn update(
        &self,
        state: &mut Self::State,
        _event: iced::widget::canvas::Event,
        bounds: iced::Rectangle,
        cursor: iced::mouse::Cursor,
    ) -> (iced::event::Status, Option<Message>) {
        if let mouse::Cursor::Available(position) = cursor {
            state.mouse_position = bounds
                .contains(position)
                .then(|| (position.x - bounds.x, position.y - bounds.y));
        }

        (iced::event::Status::Ignored, None)
    }
}

pub struct DeciamlRanged(Decimal, Decimal);
impl Ranged for DeciamlRanged {
    type FormatOption = DefaultFormatting;
    type ValueType = Decimal;

    fn map(&self, value: &Self::ValueType, limit: (i32, i32)) -> i32 {
        todo!()
    }

    fn key_points<Hint: KeyPointHint>(&self, hint: Hint) -> Vec<Self::ValueType> {
        todo!()
    }

    fn range(&self) -> Range<Self::ValueType> {
        self.0..self.1
    }
}
