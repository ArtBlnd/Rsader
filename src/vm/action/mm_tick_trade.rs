use std::sync::Arc;
use std::time::Duration;

use rust_decimal::Decimal;
use rust_decimal_macros::dec;

use crate::console;
use crate::currency::Currency;
use crate::exchange::{Exchange, OrderState, OrderToken, Unit};
use crate::ir::ty::{IrType, TypeOf};
use crate::utils::async_helpers;

use super::{Action, ActionBehavior, ActionState, TaskContext};

// Maker-Taker tick trade action
pub struct MmTickTradeAction<E> {
    pub ex: Arc<E>,
    pub currency: Currency,
    pub amount: Decimal,

    pub n_trades: usize,
    pub n_sl: usize,
    pub n_tp: usize,

    pub start_user_bal: Option<Decimal>,
    pub cached_tick_size: Decimal,
}

impl<E> TypeOf for MmTickTradeAction<E> {
    fn type_of(&self) -> IrType {
        IrType::void()
    }
}

impl<E> MmTickTradeAction<E>
where
    E: Exchange,
{
    pub fn new(ex: Arc<E>, currency: Currency, amount: Decimal, _n_trades: usize) -> Self {
        Self {
            ex,
            currency,
            amount,
            n_trades: 100,
            n_sl: 0,
            n_tp: 0,
            start_user_bal: None,
            cached_tick_size: dec!(0),
        }
    }

    #[async_recursion::async_recursion]
    pub async fn get_info(&mut self) -> (Unit, Unit, Unit, Unit, Decimal, Decimal) {
        let Ok(orderbook) = self
            .ex
            .orderbook((self.currency, Currency::KRW), None)
            .await
        else {
            async_helpers::sleep(Duration::from_millis(1000)).await;
            return self.get_info().await;
        };

        let ask0 = orderbook.asks[0].clone();
        let ask1 = orderbook.asks[1].clone();
        let bid0 = orderbook.bids[0].clone();
        let bid1 = orderbook.bids[1].clone();

        let ask_amounts: Decimal = orderbook.asks[1..5].iter().map(|x| x.amount).sum();
        let bid_amounts: Decimal = orderbook.bids[1..5].iter().map(|x| x.amount).sum();

        if self.cached_tick_size == dec!(0) {
            let diffs = orderbook.asks.windows(2).map(|x| x[1].price - x[0].price);
            let tick_size = diffs.min().unwrap();

            self.cached_tick_size = tick_size;
        }

        (ask0, ask1, bid0, bid1, ask_amounts, bid_amounts)
    }

    pub async fn tick_size(&mut self) -> Decimal {
        if self.cached_tick_size == dec!(0) {
            self.get_info().await;
        }

        assert!(self.cached_tick_size != dec!(0));
        self.cached_tick_size
    }

    pub async fn record_user_bal(&mut self) {
        let bal = self.ex.balance(Currency::KRW, None).await.unwrap();
        self.start_user_bal = Some(bal.available);
    }

    pub async fn user_balance_diff(&mut self) -> Decimal {
        let bal = self.ex.balance(Currency::KRW, None).await.unwrap();
        bal.available - self.start_user_bal.take().unwrap()
    }
}

#[derive(Default)]
pub enum MmTickTradeState {
    #[default]
    Initial,
    EnterTrade,
    WaitForExecutionBeg {
        order_token: OrderToken,
        price: Decimal,
    },
    WaitForExecutionEnd {
        order_token: OrderToken,
        price: Decimal,
    },
    Watch {
        order_token: OrderToken,
        price: Decimal,
    },
    ExecuteStopLoss {
        price: Decimal,
    },
    WaitForStopLoss {
        order_token: OrderToken,
    },
    Done,
}

impl ActionState for MmTickTradeState {
    fn state(&self) -> &'static str {
        match self {
            Self::Initial => "INITIAL",
            Self::EnterTrade => "ENTER_TRADE",
            Self::WaitForExecutionBeg { .. } => "WAIT_FOR_EXECUTION_BEG",
            Self::WaitForExecutionEnd { .. } => "WAIT_FOR_EXECUTION_END",
            Self::Watch { .. } => "WATCH",
            Self::ExecuteStopLoss { .. } => "EXECUTE_STOP_LOSS",
            Self::WaitForStopLoss { .. } => "WAIT_FOR_STOP_LOSS",
            Self::Done => "DONE",
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum MmTickTradeError {}

impl<E> Action for MmTickTradeAction<E>
where
    E: Exchange + 'static,
{
    const NAME: &'static str = "mm_tick_trade";

    type ExecutionResult = ();
    type State = MmTickTradeState;
    type Error = MmTickTradeError;

    async fn execute(
        &mut self,
        _context: &TaskContext,
        state: &mut Self::State,
    ) -> Result<ActionBehavior<Self::ExecutionResult>, Self::Error> {
        let behavior = match state {
            MmTickTradeState::Initial => {
                if self.n_trades != 0 {
                    self.n_trades -= 1;
                    *state = MmTickTradeState::EnterTrade;

                    console!(
                        "mm_tick_trade: n_trades = {:?} sl = {} tp = {}",
                        self.n_trades,
                        self.n_sl,
                        self.n_tp
                    );
                    return Ok(ActionBehavior::Continue);
                }

                ActionBehavior::Stop(())
            }
            MmTickTradeState::EnterTrade => {
                let (ask0, _, bid0, _, ask_amount, bid_amount) = self.get_info().await;

                // Safetly
                if (dec!(1) - (ask_amount / bid_amount)).abs() > dec!(0.3)
                    && ask_amount * dec!(3) > bid_amount
                {
                    return Ok(ActionBehavior::Wait(Duration::from_millis(100)));
                }

                if ask0.amount == dec!(0) || bid0.amount == dec!(0) {
                    return Ok(ActionBehavior::Wait(Duration::from_millis(100)));
                }

                let amount = (self.amount * dec!(5)).max(dec!(10000000) / bid0.price);
                if bid0.amount < amount {
                    return Ok(ActionBehavior::Wait(Duration::from_millis(100)));
                }

                let price = bid0.price;
                self.record_user_bal().await;
                let ot = self
                    .ex
                    .bid_limit((self.currency, Currency::KRW), price, self.amount, None)
                    .await
                    .unwrap();

                *state = MmTickTradeState::WaitForExecutionBeg {
                    order_token: ot,
                    price,
                };

                ActionBehavior::Continue
            }
            MmTickTradeState::WaitForExecutionBeg { order_token, price } => {
                let (_, _, bid0, _, ask_amount, bid_amount) = self.get_info().await;

                let order = self.ex.view_order(order_token).await.unwrap();
                if order.executed_volume > dec!(0) {
                    *state = MmTickTradeState::WaitForExecutionEnd {
                        order_token: order_token.clone(),
                        price: *price,
                    };

                    return Ok(ActionBehavior::Continue);
                }

                let amount = (self.amount * dec!(5)).max(dec!(10000000) / *price);
                if bid0.amount < amount
                    || bid0.price != *price
                     // Safetly
                    || ((dec!(1) - (ask_amount / bid_amount)).abs() > dec!(0.3)
                        && ask_amount * dec!(3) > bid_amount)
                {
                    if self.ex.cancel_order(order_token).await.is_err() {
                        *state = MmTickTradeState::WaitForExecutionEnd {
                            order_token: order_token.clone(),
                            price: *price,
                        }
                    } else {
                        *state = MmTickTradeState::EnterTrade;
                    }
                }

                ActionBehavior::Wait(Duration::from_millis(100))
            }
            MmTickTradeState::WaitForExecutionEnd { order_token, price } => {
                let tick_size = self.tick_size().await;

                let order = self.ex.view_order(order_token).await.unwrap();
                if order.state == OrderState::Closed {
                    console!("mm_tick_trade: executed at {:?}", price);
                    let Ok(order_token) = self
                        .ex
                        .ask_limit(
                            (self.currency, Currency::KRW),
                            *price + tick_size,
                            self.amount,
                            None,
                        )
                        .await
                    else {
                        return Ok(ActionBehavior::Wait(Duration::from_millis(100)));
                    };
                    *state = MmTickTradeState::Watch {
                        order_token,
                        price: *price,
                    };
                    return Ok(ActionBehavior::Continue);
                }

                ActionBehavior::Wait(Duration::from_millis(100))
            }
            MmTickTradeState::Watch { order_token, price } => {
                let (ask0, _, _, _, _, _) = self.get_info().await;

                let order = self.ex.view_order(order_token).await.unwrap();
                if order.state == OrderState::Closed {
                    *state = MmTickTradeState::Done;

                    self.n_tp += 1;
                    return Ok(ActionBehavior::Wait(Duration::from_millis(100)));
                }

                if order.executed_volume > dec!(0) {
                    return Ok(ActionBehavior::Wait(Duration::from_millis(100)));
                }

                if ask0.price <= *price {
                    if self.ex.cancel_order(order_token).await.is_ok() {
                        *state = MmTickTradeState::ExecuteStopLoss { price: ask0.price };
                        return Ok(ActionBehavior::Continue);
                    }
                }

                ActionBehavior::Wait(Duration::from_millis(100))
            }
            MmTickTradeState::ExecuteStopLoss { price } => {
                let Ok(order_token) = self
                    .ex
                    .ask_limit((self.currency, Currency::KRW), *price, self.amount, None)
                    .await
                else {
                    return Ok(ActionBehavior::Wait(Duration::from_millis(100)));
                };

                *state = MmTickTradeState::WaitForStopLoss { order_token };
                ActionBehavior::Continue
            }
            MmTickTradeState::WaitForStopLoss { order_token } => {
                let order = self.ex.view_order(order_token).await.unwrap();
                if order.state == OrderState::Closed {
                    self.n_sl += 1;
                    *state = MmTickTradeState::Done;
                    return Ok(ActionBehavior::Continue);
                }

                ActionBehavior::Wait(Duration::from_millis(1000))
            }
            MmTickTradeState::Done => {
                let diff = self.user_balance_diff().await;
                console!("mm_tick_trade: user balance diff = {:?}", diff);

                *state = MmTickTradeState::Initial;
                ActionBehavior::Wait(Duration::from_secs(5))
            }
        };

        Ok(behavior)
    }

    async fn execute_fallback(
        &mut self,
        _context: &TaskContext,
        _state: Self::State,
        _error: Option<Self::Error>,
    ) -> Result<(), Self::Error> {
        Ok(())
    }
}
