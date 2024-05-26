use std::error::Error as StdError;
use std::sync::Arc;

pub mod binance;
pub mod bithumb;
pub mod upbit;

use serde::{Deserialize, Serialize};

use self::{binance::Binance, bithumb::Bithumb, upbit::Upbit};
use crate::utils::broadcaster::Subscription;
use crate::{
    currency::Currency,
    utils::maybe_trait::{MaybeSend, MaybeSync},
    utils::Decimal,
};

#[cfg_attr(not(target_arch = "wasm32"), trait_variant::make(Send))]
pub trait Exchange: MaybeSync {
    const NAME: &'static str;

    type Error: StdError + MaybeSend;

    fn subscribe(
        &self,
        pair: (Currency, Currency),
        market: Option<Market>,
    ) -> Subscription<RealtimeData>;

    async fn orderbook(
        &self,
        pair: (Currency, Currency),
        market: Option<Market>,
    ) -> Result<Orderbook, Self::Error>;

    async fn candlesticks(
        &self,
        _pair: (Currency, Currency),
        _market: Option<Market>,
    ) -> Result<CandleSticks, Self::Error>;

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
    ) -> Result<(), Self::Error>;
}

pub type OrderToken = serde_json::Value;

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Default, Hash, rune::Any)]
pub enum Market {
    #[rune(constructor)]
    #[default]
    Spot,
    #[rune(constructor)]
    Future,
}

#[derive(Clone)]
pub struct Exchanges {
    pub upbit: Arc<Upbit>,
    pub binance: Arc<Binance>,
    pub bithumb: Arc<Bithumb>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash, rune::Any)]
pub struct Unit {
    #[rune(get)]
    pub price: Decimal,
    #[rune(get)]
    pub amount: Decimal,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash, rune::Any)]
pub struct Orderbook {
    #[rune(get)]
    pub pair: (Currency, Currency),
    #[rune(get)]
    pub bids: Vec<Unit>,
    #[rune(get)]
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

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash, rune::Any)]
pub struct Balance {
    #[rune(get)]
    pub available: Decimal,
    #[rune(get)]
    pub locked: Decimal,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash, rune::Any)]
pub struct Ticker {
    pub timestamp: u64,
    pub open: Decimal,
    pub close: Decimal,
    pub low: Decimal,
    pub high: Decimal,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash, rune::Any)]
pub struct CandleSticks {
    pub pair: (Currency, Currency),
    pub tickers: Vec<Ticker>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash, rune::Any)]
pub struct Trade {
    pub pair: (Currency, Currency),
    pub timestamp: i64,
    pub price: Decimal,
    pub amount: Decimal,
    pub is_bid: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash, rune::Any)]
pub struct Order {
    pub state: OrderState,
    pub executed_volume: Decimal,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash, rune::Any)]
pub enum OrderState {
    Wait,
    Closed,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash, rune::Any)]
pub enum RealtimeData {
    Orderbook(#[rune(get)] Orderbook),
    Trade(#[rune(get)] Trade),
}

pub fn execute_if<E, O>(name: &str, ex: Arc<E>, f: impl FnOnce(Arc<E>) -> O) -> Option<O>
where
    E: Exchange,
{
    if name == E::NAME {
        return Some(f(ex));
    }

    None
}

#[macro_export]
macro_rules! select_ex {
    ($ex:expr, $name:expr, $f:expr) => {
        execute_if($name.as_str(), $ex.upbit.clone(), $f)
            .or_else(|| execute_if($name.as_str(), $ex.binance.clone(), $f))
            .or_else(|| execute_if($name.as_str(), $ex.bithumb.clone(), $f))
    };
}
