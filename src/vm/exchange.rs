use std::sync::Arc;

use crate::exchange::{Exchange, Market, OrderToken};
use crate::utils::maybe_trait::MaybeSend;
use crate::utils::Decimal;
use crate::{currency::Currency, exchange::Orderbook};

use super::error::Error;

use rune::runtime::Ref;

pub fn install_module_exchange(context: &mut rune::Context) {
    let mut module = rune::Module::new();

    module.ty::<Currency>().unwrap();
    module.ty::<Orderbook>().unwrap();
    module.ty::<Market>().unwrap();
    module.ty::<ExchangeOpaque>().unwrap();

    module.function_meta(orderbook).unwrap();

    context.install(module).unwrap();
}

pub fn install_exchange<E>(context: &mut rune::Context, ex: Arc<E>)
where
    E: Exchange + MaybeSend + 'static,
{
    let mut module = rune::Module::new();
    let ex = ExchangeOpaque(ex);
    module.constant(E::NAME, ex).build().unwrap();

    context.install(module).unwrap();
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
#[cfg_attr(any(target_arch = "wasm32"), async_trait::async_trait(?Send))]
pub trait VmExchange {
    async fn orderbook(
        &self,
        pair: (Currency, Currency),
        market: Option<Market>,
    ) -> Result<Orderbook, Error>;

    async fn bid_limit(
        &self,
        pair: (Currency, Currency),
        price: Decimal,
        amount: Decimal,
        market: Option<Market>,
    ) -> Result<OrderTokenOpaque, Error>;

    async fn bid_market(
        &self,
        pair: (Currency, Currency),
        base_qty: Decimal,
        market: Option<Market>,
    ) -> Result<OrderTokenOpaque, Error>;

    async fn ask_limit(
        &self,
        pair: (Currency, Currency),
        price: Decimal,
        amount: Decimal,
        market: Option<Market>,
    ) -> Result<OrderTokenOpaque, Error>;

    async fn ask_market(
        &self,
        pair: (Currency, Currency),
        base_qty: Decimal,
        market: Option<Market>,
    ) -> Result<OrderTokenOpaque, Error>;
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
#[cfg_attr(any(target_arch = "wasm32"), async_trait::async_trait(?Send))]
impl<E> VmExchange for E
where
    E: Exchange + MaybeSend + 'static,
{
    async fn orderbook(
        &self,
        pair: (Currency, Currency),
        market: Option<Market>,
    ) -> Result<Orderbook, Error> {
        Ok(self
            .orderbook(pair, market)
            .await
            .map_err(|e| Error::from_stderr(e))?)
    }

    async fn bid_limit(
        &self,
        pair: (Currency, Currency),
        price: Decimal,
        amount: Decimal,
        market: Option<Market>,
    ) -> Result<OrderTokenOpaque, Error> {
        Ok(OrderTokenOpaque(
            self.bid_limit(pair, price, amount, market)
                .await
                .map_err(|e| Error::from_stderr(e))?,
        ))
    }

    async fn bid_market(
        &self,
        pair: (Currency, Currency),
        base_qty: Decimal,
        market: Option<Market>,
    ) -> Result<OrderTokenOpaque, Error> {
        Ok(OrderTokenOpaque(
            self.bid_market(pair, base_qty, market)
                .await
                .map_err(|e| Error::from_stderr(e))?,
        ))
    }

    async fn ask_limit(
        &self,
        pair: (Currency, Currency),
        price: Decimal,
        amount: Decimal,
        market: Option<Market>,
    ) -> Result<OrderTokenOpaque, Error> {
        Ok(OrderTokenOpaque(
            self.ask_limit(pair, price, amount, market)
                .await
                .map_err(|e| Error::from_stderr(e))?,
        ))
    }

    async fn ask_market(
        &self,
        pair: (Currency, Currency),
        base_qty: Decimal,
        market: Option<Market>,
    ) -> Result<OrderTokenOpaque, Error> {
        Ok(OrderTokenOpaque(
            self.ask_market(pair, base_qty, market)
                .await
                .map_err(|e| Error::from_stderr(e))?,
        ))
    }
}

#[allow(dead_code)]
#[derive(rune::Any, Clone)]
pub struct ExchangeOpaque(Arc<dyn VmExchange + 'static>);

#[allow(dead_code)]
#[derive(rune::Any, Clone)]
pub struct OrderTokenOpaque(OrderToken);

#[rune::function(instance)]
pub async fn orderbook(
    ex: Ref<ExchangeOpaque>,
    pair: (Currency, Currency),
    market: Option<Market>,
) -> Result<Orderbook, Error> {
    ex.0.orderbook(pair, market).await
}

#[rune::function(instance)]
pub async fn bid_limit(
    ex: Ref<ExchangeOpaque>,
    pair: (Currency, Currency),
    price: Decimal,
    amount: Decimal,
    market: Option<Market>,
) -> Result<OrderTokenOpaque, Error> {
    ex.0.bid_limit(pair, price, amount, market).await
}

#[rune::function(instance)]
pub async fn bid_market(
    ex: Ref<ExchangeOpaque>,
    pair: (Currency, Currency),
    base_qty: Decimal,
    market: Option<Market>,
) -> Result<OrderTokenOpaque, Error> {
    ex.0.bid_market(pair, base_qty, market).await
}

#[rune::function(instance)]
pub async fn ask_limit(
    ex: Ref<ExchangeOpaque>,
    pair: (Currency, Currency),
    price: Decimal,
    amount: Decimal,
    market: Option<Market>,
) -> Result<OrderTokenOpaque, Error> {
    ex.0.ask_limit(pair, price, amount, market).await
}

#[rune::function(instance)]
pub async fn ask_market(
    ex: Ref<ExchangeOpaque>,
    pair: (Currency, Currency),
    base_qty: Decimal,
    market: Option<Market>,
) -> Result<OrderTokenOpaque, Error> {
    ex.0.ask_market(pair, base_qty, market).await
}
