use rust_decimal::Decimal;
use std::{sync::Arc, time::Duration};

use crate::{
    currency::Currency,
    exchange::{Exchange, Market, OrderState as ExchangeOrderState, OrderToken},
    ir::ty::{IrType, TypeOf},
};

use super::{Action, ActionBehavior, ActionState, TaskContext};

#[derive(thiserror::Error, Debug, Clone, PartialEq, Eq)]
pub enum BasicError<E> {
    #[error("exchange error: {0}")]
    ExchangeError(#[from] E),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderType {
    BidMarket {
        quote_qty: Decimal,
        pair: (Currency, Currency),
    },
    AskMarket {
        base_qty: Decimal,
        pair: (Currency, Currency),
    },
    BidLimit {
        price: Decimal,
        pair: (Currency, Currency),
        amount: Decimal,
    },
    AskLimit {
        price: Decimal,
        pair: (Currency, Currency),
        amount: Decimal,
    },
}

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub enum OrderState {
    #[default]
    Initial,
    Waiting(OrderToken),
}

impl ActionState for OrderState {
    fn state(&self) -> &'static str {
        match self {
            OrderState::Initial => "INITIAL",
            OrderState::Waiting(_) => "WAITING",
        }
    }
}

pub struct OrderAction<E> {
    pub order_type: OrderType,
    pub exchange: Arc<E>,
    pub is_future: bool,
}

impl<E> TypeOf for OrderAction<E> {
    fn type_of(&self) -> IrType {
        IrType::decimal()
    }
}

impl<E> Action for OrderAction<E>
where
    E: Exchange + 'static,
{
    const NAME: &'static str = "bid_market";

    type ExecutionResult = Decimal;
    type State = OrderState;
    type Error = BasicError<E::Error>;

    async fn execute(
        &mut self,
        _context: &TaskContext,
        state: &mut Self::State,
    ) -> Result<ActionBehavior<Self::ExecutionResult>, Self::Error> {
        let market = if self.is_future {
            Market::Future
        } else {
            Market::Spot
        };

        let behavior = match state {
            OrderState::Initial => {
                let ot = match self.order_type {
                    OrderType::BidMarket {
                        quote_qty: price,
                        pair: target,
                    } => self.exchange.bid_market(target, price, Some(market)).await,
                    OrderType::BidLimit {
                        price,
                        pair: target,
                        amount,
                    } => {
                        self.exchange
                            .bid_limit(target, price, amount, Some(market))
                            .await
                    }
                    OrderType::AskMarket {
                        base_qty: price,
                        pair: target,
                    } => self.exchange.ask_market(target, price, Some(market)).await,
                    OrderType::AskLimit {
                        price,
                        pair: target,
                        amount,
                    } => {
                        self.exchange
                            .ask_limit(target, price, amount, Some(market))
                            .await
                    }
                }?;

                *state = OrderState::Waiting(ot);
                ActionBehavior::Wait(Duration::from_millis(100))
            }
            OrderState::Waiting(order_token) => {
                let order = self.exchange.view_order(order_token).await?;

                if order.state == ExchangeOrderState::Closed {
                    ActionBehavior::Stop(order.executed_volume)
                } else {
                    ActionBehavior::Wait(Duration::from_secs(1))
                }
            }
        };

        Ok(behavior)
    }

    async fn execute_fallback(
        &mut self,
        _context: &TaskContext,
        state: Self::State,
        _error: Option<Self::Error>,
    ) -> Result<(), Self::Error> {
        if let OrderState::Waiting(order_token) = state {
            let _ = self.exchange.cancel_order(&order_token).await?;
        }

        Ok(())
    }
}
