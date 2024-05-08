use std::error::Error as StdError;
use std::sync::Arc;

pub mod binance;
pub mod bithumb;
pub mod upbit;

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::{
    broadcast,
    currency::Currency,
    global_context::GlobalContext,
    utils::maybe_trait::{MaybeSend, MaybeSync},
};

use self::{binance::Binance, bithumb::Bithumb, upbit::Upbit};

pub type OrderToken = serde_json::Value;

#[cfg_attr(not(target_arch = "wasm32"), trait_variant::make(Send))]
pub trait Exchange: MaybeSync {
    const NAME: &'static str;

    type Error: StdError + MaybeSend;

    fn initialize(&self, global_ctx: &GlobalContext, broadcaster: broadcast::Broadcaster);
    fn subscribe(&self, pair: (Currency, Currency), market: Option<Market>);

    async fn orderbook(
        &self,
        pair: (Currency, Currency),
        market: Option<Market>,
    ) -> Result<Orderbook, Self::Error>;

    async fn balance(
        &self,
        currency: Currency,
        market: Option<Market>,
    ) -> Result<Balance, Self::Error>;

    async fn bid_limit(
        &self,
        pair: (Currency, Currency),
        price: Decimal,
        amount: Decimal,
        market: Option<Market>,
    ) -> Result<OrderToken, Self::Error>;

    async fn bid_market(
        &self,
        pair: (Currency, Currency),
        quote_qty: Decimal,
        market: Option<Market>,
    ) -> Result<OrderToken, Self::Error>;

    async fn ask_limit(
        &self,
        pair: (Currency, Currency),
        price: Decimal,
        amount: Decimal,
        market: Option<Market>,
    ) -> Result<OrderToken, Self::Error>;

    async fn ask_market(
        &self,
        pair: (Currency, Currency),
        base_qty: Decimal,
        market: Option<Market>,
    ) -> Result<OrderToken, Self::Error>;

    async fn view_order(&self, order_token: &OrderToken) -> Result<Order, Self::Error>;
    async fn wait_order(&self, order_token: &OrderToken) -> Result<Decimal, Self::Error>;
    async fn cancel_order(&self, order_token: &OrderToken) -> Result<Decimal, Self::Error>;

    async fn candlesticks(
        &self,
        _pair: (Currency, Currency),
        _market: Option<Market>,
    ) -> Result<CandleSticks, Self::Error> {
        let fut = async move { unimplemented!() };

        #[cfg(not(target_arch = "wasm32"))]
        return fut;
        #[cfg(any(target_arch = "wasm32"))]
        return fut.await;
    }

    async fn withdraw(
        &self,
        currency: Currency,
        amount: Decimal,
        address1: &str,
        address2: Option<&str>,
        network: Option<&str>,
    ) -> Result<(), Self::Error>;

    /// Set leverage for a pair.
    /// If pair is None, set leverage for all pairs.
    async fn set_leverage(
        &self,
        _pair: Option<(Currency, Currency)>,
        _value: u64,
    ) -> Result<(), Self::Error> {
        let fut = async move { unimplemented!() };

        #[cfg(not(target_arch = "wasm32"))]
        return fut;
        #[cfg(any(target_arch = "wasm32"))]
        return fut.await;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Market {
    #[default]
    Spot,
    Future,
}

#[derive(Clone)]
pub struct Exchanges {
    pub upbit: Arc<Upbit>,
    pub binance: Arc<Binance>,
    pub bithumb: Arc<Bithumb>,
}

macro_rules! select_ex {
    ($exchange_name:expr, $ex:expr, $exname:ident, $exec:expr) => {
        match $exchange_name {
            "upbit" => {
                let $exname = $ex.upbit.clone();
                $exec;
            }
            "binance" => {
                let $exname = $ex.binance.clone();
                $exec;
            }
            "bithumb" => {
                let $exname = $ex.bithumb.clone();
                $exec;
            }
            _ => panic!("Unknown exchange: {}", $exchange_name),
        }
    };
}

#[derive(
    strum::EnumString, strum::Display, strum::EnumIter, Debug, PartialEq, Eq, Clone, Copy, Hash,
)]
#[strum(serialize_all = "snake_case")]
pub enum ExchangeKind {
    Binance,
    Upbit,
    Bithumb,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unit {
    pub price: Decimal,
    pub amount: Decimal,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Orderbook {
    pub pair: (Currency, Currency),
    pub bids: Vec<Unit>,
    pub asks: Vec<Unit>,
}

impl Orderbook {
    pub fn normalize(self) -> Self {
        Orderbook {
            pair: self.pair,
            asks: self
                .asks
                .into_iter()
                .map(|unit| Unit {
                    price: unit.price.normalize(),
                    amount: unit.amount.normalize(),
                })
                .collect(),
            bids: self
                .bids
                .into_iter()
                .map(|unit| Unit {
                    price: unit.price.normalize(),
                    amount: unit.amount.normalize(),
                })
                .collect(),
        }
    }

    pub fn max_amount(&self) -> Decimal {
        self.bids
            .iter()
            .chain(self.asks.iter())
            .map(|unit| unit.amount)
            .max()
            .unwrap_or_default()
    }

    pub fn max_price(&self) -> Decimal {
        self.bids
            .iter()
            .chain(self.asks.iter())
            .map(|unit| unit.price)
            .max()
            .unwrap_or_default()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Balance {
    pub available: Decimal,
    pub locked: Decimal,
}

#[derive(Debug, Clone)]
pub struct Ticker {
    pub timestamp: u64,
    pub open: Decimal,
    pub close: Decimal,
    pub low: Decimal,
    pub high: Decimal,
}

#[derive(Debug, Clone)]
pub struct CandleSticks {
    pub pair: (Currency, Currency),
    pub tickers: Vec<Ticker>,
}

#[derive(Debug, Clone)]
pub struct Trade {
    pub pair: (Currency, Currency),
    pub timestamp: u64,
    pub price: Decimal,
    pub qty: Decimal,
    pub is_bid: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Order {
    pub state: OrderState,
    pub executed_volume: Decimal,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum OrderState {
    Wait,
    Closed,
}
