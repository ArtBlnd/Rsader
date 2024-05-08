use std::any::Any;
use std::fmt::Debug;
use std::sync::Arc;

use crate::exchange::{Exchange, Market};
use crate::global_context::GlobalContext;

use crate::utils::maybe_trait::MaybeSend;
use crate::{currency::Currency, exchange::Orderbook};

use super::error::Error;

pub fn install_exchange_utils(context: &mut rune::Context) {
    let mut module = rune::Module::new();

    module.ty::<Currency>().unwrap();
    module.ty::<Orderbook>().unwrap();
    module.ty::<Market>().unwrap();
    module.ty::<ExchangeOpaque>().unwrap();

    module.function_meta(ExchangeOpaque::orderbook).unwrap();

    context.install(module).unwrap();
}

pub fn install_exchange<E>(context: &mut rune::Context, ex: E)
where
    E: Exchange + MaybeSend + 'static,
{
    let mut module = rune::Module::new();
    let ex = ExchangeOpaque(Arc::new(ex));
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
}

#[derive(rune::Any, Clone)]
pub struct ExchangeOpaque(Arc<dyn VmExchange + Send + Sync + 'static>);

impl ExchangeOpaque {
    #[rune::function(instance)]
    pub async fn orderbook(
        self,
        pair: (Currency, Currency),
        market: Option<Market>,
    ) -> Result<Orderbook, Error> {
        self.0.orderbook(pair, market).await
    }
}
