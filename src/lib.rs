use std::sync::Arc;

mod currency;
mod global_context;
#[macro_use]
mod exchange;
mod broadcast;
mod config;
#[macro_use]
mod utils;
mod vm;
mod websocket;

use exchange::{binance::Binance, bithumb::Bithumb, upbit::Upbit, Exchange, Exchanges};
