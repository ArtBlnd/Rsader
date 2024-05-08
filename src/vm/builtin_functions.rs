use strum::IntoEnumIterator;

use crate::{
    exchange::{Exchange, Market},
    ir::ty::{CompositeTypes, IrType, PrimitiveTypes},
    vm::{
        action::{ActionExecutor, ActionFuture, MmTickTradeAction, OrderAction, OrderType},
        value::IntoValue,
    },
};

use super::{
    action::{TaskContext, TickTradeAction},
    builtin_types,
    context::VmContext,
    function::Function,
    value::Value,
    VmError,
};

#[derive(Clone, Debug, PartialEq, Eq, Hash, strum::EnumIter, strum::EnumString, strum::Display)]
#[strum(serialize_all = "snake_case")]
pub enum BuiltinFunction {
    BidLimit,
    BidMarket,
    AskLimit,
    AskMarket,
    FbidLimit,
    FbidMarket,
    FaskLimit,
    FaskMarket,
    Withdraw,
    Balance,
    Orderbook,
    TickTrade,
    MmTickTrade,
}

impl BuiltinFunction {
    pub fn ty(&self) -> IrType {
        IrType::Composite(match self {
            // function signature `function(Exchange, (Currency, Currency), Decimal, Decimal) -> Decimal`
            BuiltinFunction::AskLimit
            | BuiltinFunction::BidLimit
            | BuiltinFunction::FaskLimit
            | BuiltinFunction::FbidLimit => CompositeTypes::Function {
                args: vec![
                    builtin_types::exchange(),
                    IrType::tuple([IrType::currency(), IrType::currency()]),
                    IrType::decimal(),
                    IrType::decimal(),
                ],
                generics: vec![],
                ret: Box::new(IrType::Primitive(PrimitiveTypes::Decimal)),
            },

            // function signature `function(Exchange, (Currency, Currency), Decimal) -> Decimal`
            BuiltinFunction::AskMarket
            | BuiltinFunction::BidMarket
            | BuiltinFunction::FaskMarket
            | BuiltinFunction::FbidMarket => CompositeTypes::Function {
                args: vec![
                    builtin_types::exchange(),
                    IrType::tuple([IrType::currency(), IrType::currency()]),
                    IrType::decimal(),
                ],
                generics: vec![],
                ret: Box::new(IrType::Primitive(PrimitiveTypes::Decimal)),
            },

            // function signature `function(Exchange, Currency, Decimal, String, Currency) -> void`
            BuiltinFunction::Withdraw => CompositeTypes::Function {
                args: vec![
                    builtin_types::exchange(),
                    IrType::currency(),
                    IrType::decimal(),
                    IrType::string(),
                    IrType::currency(),
                ],
                generics: vec![],
                ret: Box::new(IrType::void()),
            },

            // function signature `function(Exchange, Currency) -> Decimal`
            BuiltinFunction::Balance => CompositeTypes::Function {
                args: vec![builtin_types::exchange(), IrType::currency()],
                generics: vec![],
                ret: Box::new(IrType::decimal()),
            },

            // function signature `function(Exchange, (Currency, Currency)) -> void`
            BuiltinFunction::Orderbook => CompositeTypes::Function {
                args: vec![
                    builtin_types::exchange(),
                    IrType::tuple([IrType::currency(), IrType::currency()]),
                ],
                generics: vec![],
                ret: Box::new(IrType::void()),
            },

            // function signature `function(Currency, Decimal, Decimal) -> void`
            BuiltinFunction::TickTrade | BuiltinFunction::MmTickTrade => CompositeTypes::Function {
                args: vec![IrType::currency(), IrType::decimal(), IrType::decimal()],
                generics: vec![],
                ret: Box::new(IrType::void()),
            },
        })
    }
}

pub fn register_builtin_functions(context: &mut VmContext) {
    for builtin in BuiltinFunction::iter() {
        context.set_variable(
            builtin.to_string().as_str(),
            Value::Function(Function::Builtin(builtin)),
        );
    }
}

pub async fn execute_builtins(
    builtin: BuiltinFunction,
    ctx: &mut VmContext,
    task_ctx: &TaskContext,
    _args0: Vec<IrType>,
    args1: Vec<Value>,
) -> Result<Option<Value>, VmError> {
    Ok(match builtin {
        BuiltinFunction::BidLimit => handle_bid_limit(ctx, task_ctx, &args1, false).await,
        BuiltinFunction::BidMarket => handle_bid_market(ctx, task_ctx, &args1, false).await,
        BuiltinFunction::AskLimit => handle_ask_limit(ctx, task_ctx, &args1, false).await,
        BuiltinFunction::AskMarket => handle_ask_market(ctx, task_ctx, &args1, false).await,
        BuiltinFunction::FbidLimit => handle_bid_limit(ctx, task_ctx, &args1, true).await,
        BuiltinFunction::FbidMarket => handle_bid_market(ctx, task_ctx, &args1, true).await,
        BuiltinFunction::FaskLimit => handle_ask_limit(ctx, task_ctx, &args1, true).await,
        BuiltinFunction::FaskMarket => handle_ask_market(ctx, task_ctx, &args1, true).await,
        BuiltinFunction::Withdraw => handle_withdraw(ctx, task_ctx, &args1).await,
        BuiltinFunction::Balance => handle_balance(ctx, task_ctx, &args1, false).await,
        BuiltinFunction::Orderbook => handle_orderbook(ctx, task_ctx, &args1).await,
        BuiltinFunction::TickTrade => handle_tick_trade(ctx, task_ctx, &args1).await,
        BuiltinFunction::MmTickTrade => handle_mm_tick_trade(ctx, task_ctx, &args1).await,
    })
}

fn exname_from_value<'x>(value: &'x Value) -> &'x str {
    let Value::Struct(fields, _) = value else {
        panic!("invalid argument for exname_from_value");
    };

    fields.values().next().unwrap().as_str().unwrap()
}

async fn handle_bid_limit(
    ctx: &mut VmContext,
    task_ctx: &TaskContext,
    args: &[Value],
    is_future: bool,
) -> Option<Value> {
    use crate::vm::value::Value::*;
    let (exchange, pair, &price, &amount) = {
        let [exchange, Tuple(pair), Decimal(price), Decimal(amount)] = args else {
            panic!("invalid argument for bid_limit");
        };
        (
            exname_from_value(exchange),
            (
                pair[0].as_currency().unwrap(),
                pair[1].as_currency().unwrap(),
            ),
            price,
            amount,
        )
    };

    select_ex!(exchange, ctx.global_ctx().ex(), ex, {
        return ActionExecutor::new(OrderAction {
            order_type: OrderType::BidLimit {
                price,
                amount,
                pair,
            },
            exchange: ex,
            is_future,
        })
        .execute(task_ctx.clone())
        .await;
    });
}

async fn handle_bid_market(
    ctx: &mut VmContext,
    task_ctx: &TaskContext,
    args: &[Value],
    is_future: bool,
) -> Option<Value> {
    use crate::vm::value::Value::*;
    let (exchange, pair, &qty) = {
        let [exchange, Tuple(pair), Decimal(qty)] = args else {
            panic!("invalid argument for bid_limit");
        };
        (
            exname_from_value(exchange),
            (
                pair[0].as_currency().unwrap(),
                pair[1].as_currency().unwrap(),
            ),
            qty,
        )
    };

    select_ex!(exchange, ctx.global_ctx().ex(), ex, {
        return ActionExecutor::new(OrderAction {
            order_type: OrderType::BidMarket {
                quote_qty: qty,
                pair,
            },
            exchange: ex,
            is_future,
        })
        .execute(task_ctx.clone())
        .await;
    })
}

async fn handle_ask_limit(
    ctx: &mut VmContext,
    task_ctx: &TaskContext,
    args: &[Value],
    is_future: bool,
) -> Option<Value> {
    use crate::vm::value::Value::*;
    let (exchange, pair, &price, &amount) = {
        let [exchange, Tuple(pair), Decimal(price), Decimal(amount)] = args else {
            panic!("invalid argument for bid_limit");
        };
        (
            exname_from_value(exchange),
            (
                pair[0].as_currency().unwrap(),
                pair[1].as_currency().unwrap(),
            ),
            price,
            amount,
        )
    };

    select_ex!(exchange, ctx.global_ctx().ex(), ex, {
        return ActionExecutor::new(OrderAction {
            order_type: OrderType::AskLimit {
                price,
                amount,
                pair,
            },
            exchange: ex,
            is_future,
        })
        .execute(task_ctx.clone())
        .await;
    });
}

async fn handle_ask_market(
    ctx: &mut VmContext,
    task_ctx: &TaskContext,
    args: &[Value],
    is_future: bool,
) -> Option<Value> {
    use crate::vm::value::Value::*;
    let (exchange, pair, &qty) = {
        let [exchange, Tuple(pair), Decimal(qty)] = args else {
            panic!("invalid argument for bid_limit");
        };
        (
            exname_from_value(exchange),
            (
                pair[0].as_currency().unwrap(),
                pair[1].as_currency().unwrap(),
            ),
            qty,
        )
    };

    select_ex!(exchange, ctx.global_ctx().ex(), ex, {
        return ActionExecutor::new(OrderAction {
            order_type: OrderType::AskMarket {
                base_qty: qty,
                pair,
            },
            exchange: ex,
            is_future,
        })
        .execute(task_ctx.clone())
        .await;
    });
}

async fn handle_withdraw(
    ctx: &mut VmContext,
    task_ctx: &TaskContext,
    args: &[Value],
) -> Option<Value> {
    use crate::vm::value::Value::*;
    let (exchange, currency, &amount, address, network) = {
        match args {
            [exchange, Currency(pair), Decimal(amount), Str(address), Currency(network)] => (
                exname_from_value(exchange),
                pair.clone(),
                amount,
                address.clone(),
                Some(network),
            ),
            _ => panic!("invalid argument for withdraw"),
        }
    };

    let mut address = address.as_str().split('#').map(|v| v.to_string());
    let address1 = address.next().unwrap();
    let address2 = address.next();

    let network = network.map(|n| n.to_string());

    select_ex!(exchange, ctx.global_ctx().ex(), ex, {
        return ActionExecutor::new(ActionFuture::new(
            async move {
                ex.withdraw(
                    currency,
                    amount,
                    &address1,
                    address2.as_deref(),
                    network.as_deref(),
                )
                .await
                .unwrap()
            },
            IrType::boolean(),
        ))
        .execute(task_ctx.clone())
        .await;
    });
}

async fn handle_balance(
    ctx: &mut VmContext,
    task_ctx: &TaskContext,
    args: &[Value],
    is_future: bool,
) -> Option<Value> {
    use crate::vm::value::Value::*;
    let (exchange, currency) = {
        let [exchange, Currency(currency)] = args else {
            panic!("invalid argument for balance");
        };
        (exname_from_value(exchange), currency.clone())
    };

    let market = if is_future {
        Some(Market::Future)
    } else {
        Some(Market::Spot)
    };

    select_ex!(exchange, ctx.global_ctx().ex(), ex, {
        return ActionExecutor::new(ActionFuture::new(
            async move {
                match ex.balance(currency, market).await {
                    Ok(balance) => balance.available,
                    Err(e) => {
                        tracing::error!("failed to fetch balance: {}", e);
                        todo!()
                    }
                }
            },
            IrType::decimal(),
        ))
        .execute(task_ctx.clone())
        .await;
    });
}

async fn handle_orderbook(
    _ctx: &mut VmContext,
    _task_ctx: &TaskContext,
    _args: &[Value],
) -> Option<Value> {
    todo!()
}

async fn handle_tick_trade(
    ctx: &mut VmContext,
    task_ctx: &TaskContext,
    args: &[Value],
) -> Option<Value> {
    use crate::vm::value::Value::*;
    let (currency, amount, _) = {
        match args {
            &[Currency(currency), Decimal(amount), Decimal(count)] => {
                (currency.clone(), amount, count)
            }
            _ => panic!("invalid argument for tick_trade"),
        }
    };

    return ActionExecutor::new(TickTradeAction::new(
        ctx.global_ctx().ex().bithumb.clone(),
        currency,
        amount,
        1,
    ))
    .execute(task_ctx.clone())
    .await;
}

async fn handle_mm_tick_trade(
    ctx: &mut VmContext,
    task_ctx: &TaskContext,
    args: &[Value],
) -> Option<Value> {
    use crate::vm::value::Value::*;
    let (currency, amount, _) = {
        match args {
            &[Currency(currency), Decimal(amount), Decimal(count)] => {
                (currency.clone(), amount, count)
            }
            _ => panic!("invalid argument for mm_tick_trade"),
        }
    };

    return ActionExecutor::new(MmTickTradeAction::new(
        ctx.global_ctx().ex().bithumb.clone(),
        currency,
        amount,
        1,
    ))
    .execute(task_ctx.clone())
    .await;
}
