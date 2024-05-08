use std::sync::Arc;
use std::time::Duration;

use rust_decimal::Decimal;
use rust_decimal_macros::dec;

use crate::console;
use crate::currency::Currency;
use crate::exchange::{Exchange, OrderState, OrderToken, Unit};
use crate::ir::ty::{IrType, TypeOf};

use super::{Action, ActionBehavior, ActionState, TaskContext};

pub struct TickTradeAction<E> {
    pub ex: Arc<E>,
    pub currency: Currency,
    pub amount: Decimal,

    pub n_trades: usize,
    pub sl_count: usize,
    pub tp_count: usize,

    pub start_user_bal: Option<Decimal>,

    pub cached_tick_size: Decimal,
}

impl<E> TypeOf for TickTradeAction<E> {
    fn type_of(&self) -> IrType {
        IrType::void()
    }
}

impl<E> TickTradeAction<E>
where
    E: Exchange,
{
    pub fn new(ex: Arc<E>, currency: Currency, amount: Decimal, _n_trades: usize) -> Self {
        Self {
            ex,
            currency,
            amount,
            n_trades: 100,
            sl_count: 0,
            tp_count: 0,
            start_user_bal: None,
            cached_tick_size: dec!(0),
        }
    }

    pub async fn get_info(&mut self) -> (Unit, Unit, Unit, Unit, Decimal, Decimal) {
        let orderbook = self
            .ex
            .orderbook((self.currency, Currency::KRW), None)
            .await
            .unwrap();

        let ask0 = orderbook.asks[0].clone();
        let ask1 = orderbook.asks[1].clone();
        let bid0 = orderbook.bids[0].clone();
        let bid1 = orderbook.bids[1].clone();

        let ask_amounts: Decimal = orderbook.asks[1..4].iter().map(|x| x.amount).sum();
        let bid_amounts: Decimal = orderbook.bids[1..4].iter().map(|x| x.amount).sum();

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
pub enum TickTradeState {
    #[default]
    Initial,
    PositionCheck,
    WaitForExecutionBeg {
        price: Decimal,
        bid0_amount: Decimal,
        ot: OrderToken,
    },
    WaitForExecutionEnd {
        price: Decimal,
        ot: OrderToken,
    },
    Ask {
        amount: Decimal,
        price: Decimal,
    },
    Watch {
        ot: OrderToken,
        amount: Decimal,
        price_sl: Decimal,
    },
    WaitForStopLoss {
        ot: OrderToken,
    },
    Done,
}

impl ActionState for TickTradeState {
    fn state(&self) -> &'static str {
        match self {
            TickTradeState::Initial => "INITIAL",
            TickTradeState::PositionCheck => "POSITION_CHECK",
            TickTradeState::WaitForExecutionBeg { .. } => "WAIT_FOR_EXECUTION",
            TickTradeState::WaitForExecutionEnd { .. } => "WAIT_FOR_ORDER",
            TickTradeState::Ask { .. } => "ASK",
            TickTradeState::Watch { .. } => "WATCH",
            TickTradeState::WaitForStopLoss { .. } => "WAIT_FOR_STOP_LOSS",
            TickTradeState::Done => "DONE",
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum TickTradeError {}

impl<E> Action for TickTradeAction<E>
where
    E: Exchange + 'static,
{
    const NAME: &'static str = "tick_trade";

    type ExecutionResult = ();
    type State = TickTradeState;
    type Error = TickTradeError;

    async fn execute(
        &mut self,
        _context: &TaskContext,
        state: &mut Self::State,
    ) -> Result<ActionBehavior<Self::ExecutionResult>, Self::Error> {
        let behavior = match state {
            TickTradeState::Initial => {
                if self.n_trades != 0 {
                    self.n_trades -= 1;
                    *state = TickTradeState::PositionCheck;

                    console!("tick_trade: n_trades = {:?}", self.n_trades);
                    return Ok(ActionBehavior::Continue);
                }

                ActionBehavior::Stop(())
            }
            TickTradeState::PositionCheck => {
                let (ask0, ask1, mut bid0, bid1, ask_amounts, bid_amounts) = self.get_info().await;
                bid0.amount = bid0.amount.max(self.amount * dec!(1.5));

                if ask0.amount == dec!(0)
                    || ask1.amount == dec!(0)
                    || bid0.amount == dec!(0)
                    || bid1.amount == dec!(0)
                {
                    return Ok(ActionBehavior::Wait(Duration::from_millis(100)));
                }

                if bid0.price + self.tick_size().await != ask0.price {
                    return Ok(ActionBehavior::Wait(Duration::from_millis(100)));
                }

                let bid_ratio = bid1.amount / bid0.amount;
                if bid_ratio < dec!(3) || (bid0.amount / ask0.amount) > dec!(1.2) {
                    return Ok(ActionBehavior::Wait(Duration::from_millis(100)));
                }

                let ratio = bid_amounts / bid0.amount;
                if ask_amounts * dec!(1.5) < bid_amounts
                    && dec!(1) - (bid0.amount / ask0.amount).abs() < dec!(0.2)
                    && bid1.amount > bid0.amount * dec!(2.5)
                    && ratio > dec!(6)
                {
                    let price = bid0.price;

                    self.record_user_bal().await;
                    let ot = self
                        .ex
                        .bid_limit((self.currency, Currency::KRW), price, self.amount, None)
                        .await
                        .unwrap();

                    *state = TickTradeState::WaitForExecutionBeg {
                        price,
                        ot,
                        bid0_amount: bid0.amount,
                    };
                }

                ActionBehavior::Continue
            }
            TickTradeState::WaitForExecutionBeg {
                price,
                bid0_amount,
                ot,
            } => {
                let order = self.ex.view_order(ot).await.unwrap();
                if order.executed_volume > dec!(0) {
                    *state = TickTradeState::WaitForExecutionEnd {
                        price: *price,
                        ot: ot.clone(),
                    };

                    return Ok(ActionBehavior::Continue);
                }

                let (ask0, _, mut bid0, bid1, ask_amounts, bid_amounts) = self.get_info().await;
                bid0.amount = bid0.amount.max(self.amount * dec!(1.5));

                if ask0.amount == dec!(0) || bid0.amount == dec!(0) {
                    return Ok(ActionBehavior::Wait(Duration::from_millis(100)));
                }

                if ask_amounts * dec!(1.5) < bid_amounts
                    && bid0.amount > ask0.amount * dec!(0.8)
                    && bid1.amount > *bid0_amount * dec!(2.5)
                    && bid0.price == *price
                {
                    return Ok(ActionBehavior::Wait(Duration::from_millis(100)));
                }

                if self.ex.cancel_order(ot).await.is_err() {
                    *state = TickTradeState::WaitForExecutionEnd {
                        price: *price,
                        ot: ot.clone(),
                    };
                } else {
                    *state = TickTradeState::PositionCheck;
                }

                ActionBehavior::Continue
            }
            TickTradeState::WaitForExecutionEnd { price, ot } => {
                let order = self.ex.view_order(ot).await.unwrap();
                if order.state == OrderState::Closed {
                    *state = TickTradeState::Ask {
                        price: *price,
                        amount: order.executed_volume,
                    };

                    ActionBehavior::Continue
                } else {
                    ActionBehavior::Wait(Duration::from_millis(100))
                }
            }
            TickTradeState::Ask { price, amount } => {
                let tick_size = self.tick_size().await;
                let price_sl = *price - tick_size * dec!(4);
                let price_tp = *price + tick_size * dec!(1);

                assert!(*price > price_sl);
                assert!(*price < price_tp);

                console!(
                    "tick_trade: executed (sl = {:?}, tp = {:?})",
                    price_sl,
                    price_tp
                );

                let Ok(ot) = self
                    .ex
                    .ask_limit((self.currency, Currency::KRW), price_tp, *amount, None)
                    .await
                else {
                    return Ok(ActionBehavior::Wait(Duration::from_millis(100)));
                };

                *state = TickTradeState::Watch {
                    ot,
                    amount: *amount,
                    price_sl,
                };

                ActionBehavior::Continue
            }
            TickTradeState::Watch {
                ot,
                amount,
                price_sl,
            } => {
                let (ask0, _, _, _, _, _) = self.get_info().await;

                let order = self.ex.view_order(ot).await.unwrap();
                if order.state == OrderState::Closed {
                    *state = TickTradeState::Done;
                    return Ok(ActionBehavior::Continue);
                }

                if ask0.price <= *price_sl {
                    let amount_left = *amount - order.executed_volume;
                    let _ = self.ex.cancel_order(ot).await;
                    let Ok(ot) = self
                        .ex
                        .ask_limit(
                            (self.currency, Currency::KRW),
                            ask0.price,
                            amount_left,
                            None,
                        )
                        .await
                    else {
                        return Ok(ActionBehavior::Wait(Duration::from_millis(100)));
                    };

                    *state = TickTradeState::WaitForStopLoss { ot };
                    return Ok(ActionBehavior::Continue);
                }

                self.tp_count += 1;
                ActionBehavior::Wait(Duration::from_millis(100))
            }
            TickTradeState::WaitForStopLoss { ot } => {
                let order = self.ex.view_order(ot).await.unwrap();
                if order.state == OrderState::Closed {
                    *state = TickTradeState::Done;
                    return Ok(ActionBehavior::Continue);
                }

                self.sl_count += 1;
                ActionBehavior::Wait(Duration::from_millis(100))
            }
            TickTradeState::Done => {
                let diff = self.user_balance_diff().await;
                console!("tick_trade: profit = {:?}", diff);

                *state = TickTradeState::Initial;
                ActionBehavior::Wait(Duration::from_secs(5))
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
        match state {
            TickTradeState::WaitForExecutionBeg { ot, .. } => {
                let _ = self.ex.cancel_order(&ot).await;
            }

            _ => {}
        }

        Ok(())
    }
}
