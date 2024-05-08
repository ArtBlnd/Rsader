use std::{collections::HashSet, str::FromStr, sync::Arc, time::Duration};

use parking_lot::RwLock;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use serde_json::json;
use unwrap_let::unwrap_let;

use crate::{
    broadcast::{self, BroadcastFrom},
    config::Config,
    currency::{Currency, CurrencyPairDelimiterStringifier, CurrencyPairStringifier},
    exchange::{Balance, Order, OrderState, Unit},
    global_context::GlobalContext,
    utils::{
        async_helpers,
        http_client::{http_client, Client},
    },
    websocket::Websocket,
};

use super::{Exchange, Market, OrderToken, Orderbook};

fn access_key() -> Result<&'static str, UpbitError> {
    Config::get()
        .upbit
        .as_ref()
        .map(|config| config.access_key.as_str())
        .ok_or(UpbitError::ConfigNotFound)
}

fn secret_key() -> Result<&'static str, UpbitError> {
    Config::get()
        .upbit
        .as_ref()
        .map(|config| config.secret_key.as_str())
        .ok_or(UpbitError::ConfigNotFound)
}

fn gen_jwt_token(access_key: &str, secret_key: &str, body_qs: &str) -> String {
    use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
    use sha2::{Digest, Sha512};

    let body_hash = hex::encode(Sha512::digest(body_qs.as_bytes()));

    let paylaod = json!({
        "access_key": access_key,
        "nonce": chrono::Utc::now().timestamp_millis(),
        "query_hash": body_hash,
        "query_hash_alg": "SHA512",
    });

    let header = Header::new(Algorithm::HS256);
    let key = EncodingKey::from_secret(secret_key.as_ref());
    format!("Bearer {}", encode(&header, &paylaod, &key).unwrap())
}

#[derive(thiserror::Error, Debug)]
pub enum UpbitError {
    #[error("failed to get orderbook")]
    FailedToGetOrderbook,

    #[error("http client error")]
    HttpClientError(#[from] reqwest::Error),

    #[error("json error")]
    JsonError(#[from] serde_json::Error),

    #[error("bid/ask order failed")]
    OrderFailed,

    #[error("view order failed")]
    ViewOrderFailed,

    #[error("cancel order failed")]
    CancelOrderFailed,

    #[error("withdraw failed")]
    WithdrawFailed,

    #[error("cofnig not found")]
    ConfigNotFound,
}

pub struct Upbit {
    subscriptions: Arc<RwLock<HashSet<(Currency, Currency)>>>,
    http_client: Client,
}

impl Upbit {
    pub fn new() -> Self {
        Self {
            subscriptions: Arc::new(RwLock::new(HashSet::new())),
            http_client: http_client(),
        }
    }
}

impl Exchange for Upbit {
    const NAME: &'static str = "upbit";

    type Error = UpbitError;

    fn initialize(&self, global_ctx: &GlobalContext, broadcaster: broadcast::Broadcaster) {
        tracing::info!("Upbit::initialize()");
        global_ctx.spawn(spawn_ws_broadcaster(
            global_ctx.clone(),
            broadcaster,
            self.subscriptions.clone(),
        ));
    }

    fn subscribe(&self, pair: (Currency, Currency), _market: Option<Market>) {
        tracing::info!("Upbit::subscribe({:?})", pair);
        self.subscriptions.write().insert(pair);
    }

    async fn orderbook(
        &self,
        pair: (Currency, Currency),
        _market: Option<Market>,
    ) -> Result<Orderbook, Self::Error> {
        tracing::debug!("Upbit::orderbook({:?})", pair);

        let pair_stringified =
            CurrencyPairDelimiterStringifier::<'-'>::stringify(pair.1, pair.0).unwrap();
        let response = self
            .http_client
            .get(&format!(
                "https://api.upbit.com/v1/orderbook?markets={}",
                pair_stringified
            ))
            .send()
            .await?;

        #[derive(Deserialize)]
        struct Response {
            pub orderbook_units: Vec<UpbitOrderbookUnit>,
        }

        let status = response.status();
        let response = response.text().await?;
        tracing::debug!("Upbit::orderbook() response: {}", response);
        if !status.is_success() {
            return Err(UpbitError::FailedToGetOrderbook);
        }

        let response: Vec<Response> = serde_json::from_str(&response)?;
        let mut bids = Vec::new();
        let mut asks = Vec::new();

        for unit in &response[0].orderbook_units {
            let bid = Unit {
                price: unit.bid_price,
                amount: unit.bid_size,
            };
            let ask = Unit {
                price: unit.ask_price,
                amount: unit.ask_size,
            };

            bids.push(bid);
            asks.push(ask);
        }

        Ok(Orderbook { pair, bids, asks })
    }

    async fn balance(
        &self,
        currency: Currency,
        _market: Option<Market>,
    ) -> Result<Balance, Self::Error> {
        tracing::debug!("Upbit::balance({:?})", currency);

        let response = self
            .http_client
            .get("https://api.upbit.com/v1/accounts")
            .header(
                "Authorization",
                gen_jwt_token(access_key()?, secret_key()?, ""),
            )
            .send()
            .await?;

        #[derive(Deserialize)]
        struct Response {
            pub currency: String,
            pub balance: Decimal,
            pub locked: Decimal,
        }

        let status = response.status();
        let response = response.text().await?;
        tracing::debug!("Upbit::balance() response: {}", response);
        if !status.is_success() {
            todo!("handle error")
        }

        let response: Vec<Response> = serde_json::from_str(&response).unwrap();
        let response = response
            .into_iter()
            .find(|r| r.currency == currency.to_string())
            .unwrap_or_else(|| Response {
                currency: currency.to_string(),
                balance: dec!(0),
                locked: dec!(0),
            });

        Ok(Balance {
            available: response.balance,
            locked: response.locked,
        })
    }

    async fn bid_limit(
        &self,
        pair: (Currency, Currency),
        price: Decimal,
        amount: Decimal,
        _market: Option<Market>,
    ) -> Result<OrderToken, Self::Error> {
        tracing::info!("Upbit::bid_limit({:?}, {}, {})", pair, price, amount);

        let pair = CurrencyPairDelimiterStringifier::<'-'>::stringify(pair.1, pair.0).unwrap();
        let message = json!({
            "side": "bid",
            "market": pair,
            "price": price.to_string(),
            "volume": amount.to_string(),
            "ord_type": "limit",
        });

        #[derive(Serialize, Deserialize, Debug)]
        struct Response {
            pub uuid: String,
        }

        let query_string = serde_qs::to_string(&message).unwrap();
        let response = self
            .http_client
            .post("https://api.upbit.com/v1/orders")
            .header(
                "Authorization",
                gen_jwt_token(access_key()?, secret_key()?, &query_string),
            )
            .body(serde_json::to_string(&message).unwrap())
            .send()
            .await?;

        let status = response.status();
        let text = response.text().await?;

        tracing::info!("Upbit::bid_limit() response: {}", text);
        if !status.is_success() {
            return Err(UpbitError::OrderFailed);
        }

        async_helpers::sleep(Duration::from_millis(250)).await;
        let response: Response = serde_json::from_str(&text)?;
        Ok(OrderToken::String(response.uuid))
    }

    async fn bid_market(
        &self,
        pair: (Currency, Currency),
        quote_qty: Decimal,
        _market: Option<Market>,
    ) -> Result<OrderToken, Self::Error> {
        tracing::info!("Upbit::bid_market({:?}, {})", pair, quote_qty);

        let pair = CurrencyPairDelimiterStringifier::<'-'>::stringify(pair.1, pair.0).unwrap();
        let message = json!({
            "side": "bid",
            "market": pair,
            "ord_type": "price",
            "price": quote_qty.to_string(),
        });

        #[derive(Serialize, Deserialize, Debug)]
        struct Response {
            pub uuid: String,
        }

        let query_string = serde_qs::to_string(&message).unwrap();
        let response = self
            .http_client
            .post("https://api.upbit.com/v1/orders")
            .header(
                "Authorization",
                gen_jwt_token(access_key()?, secret_key()?, &query_string),
            )
            .body(serde_json::to_string(&message).unwrap())
            .send()
            .await?;

        let status = response.status();
        let text = response.text().await?;

        tracing::info!("Upbit::bid_market() response: {}", text);
        if !status.is_success() {
            return Err(UpbitError::OrderFailed);
        }

        async_helpers::sleep(Duration::from_millis(250)).await;
        let response: Response = serde_json::from_str(&text)?;
        Ok(OrderToken::String(response.uuid))
    }

    async fn ask_limit(
        &self,
        pair: (Currency, Currency),
        price: Decimal,
        amount: Decimal,
        _market: Option<Market>,
    ) -> Result<OrderToken, Self::Error> {
        tracing::info!("Upbit::ask_limit({:?}, {}, {})", pair, price, amount);

        let pair = CurrencyPairDelimiterStringifier::<'-'>::stringify(pair.1, pair.0).unwrap();
        let message = json!({
            "side": "ask",
            "market": pair,
            "price": price.to_string(),
            "volume": amount.to_string(),
            "ord_type": "limit",
        });

        #[derive(Serialize, Deserialize, Debug)]
        struct Response {
            pub uuid: String,
        }

        let query_string = serde_qs::to_string(&message).unwrap();
        let response = self
            .http_client
            .post("https://api.upbit.com/v1/orders")
            .header(
                "Authorization",
                gen_jwt_token(access_key()?, secret_key()?, &query_string),
            )
            .body(serde_json::to_string(&message).unwrap())
            .send()
            .await?;

        let status = response.status();
        let text = response.text().await?;

        tracing::info!("Upbit::ask_limit() response: {}", text);
        if !status.is_success() {
            return Err(UpbitError::OrderFailed);
        }

        async_helpers::sleep(Duration::from_millis(250)).await;
        let response: Response = serde_json::from_str(&text)?;
        Ok(OrderToken::String(response.uuid))
    }

    async fn ask_market(
        &self,
        pair: (Currency, Currency),
        base_qty: Decimal,
        _market: Option<Market>,
    ) -> Result<OrderToken, Self::Error> {
        tracing::info!("Upbit::ask_market({:?}, {})", pair, base_qty);

        let pair = CurrencyPairDelimiterStringifier::<'-'>::stringify(pair.1, pair.0).unwrap();
        let message = json!({
            "side": "ask",
            "market": pair,
            "ord_type": "market",
            "volume": base_qty.to_string(),
        });

        #[derive(Serialize, Deserialize, Debug)]
        struct Response {
            pub uuid: String,
        }

        let query_string = serde_qs::to_string(&message).unwrap();
        let response = self
            .http_client
            .post("https://api.upbit.com/v1/orders")
            .header(
                "Authorization",
                gen_jwt_token(access_key()?, secret_key()?, &query_string),
            )
            .body(serde_json::to_string(&message).unwrap())
            .send()
            .await?;

        let status = response.status();
        let text = response.text().await?;

        tracing::info!("Upbit::ask_market() response: {}", text);
        if !status.is_success() {
            return Err(UpbitError::ViewOrderFailed);
        }

        async_helpers::sleep(Duration::from_millis(250)).await;
        let response: Response = serde_json::from_str(&text)?;
        Ok(OrderToken::String(response.uuid))
    }

    async fn view_order(&self, order_token: &OrderToken) -> Result<Order, Self::Error> {
        unwrap_let!(OrderToken::String(order_token) = order_token);
        let payload = json!({
            "uuid": order_token,
        });

        let query_string = serde_qs::to_string(&payload).unwrap();
        let response = self
            .http_client
            .get(&format!(
                "https://api.upbit.com/v1/order?{}",
                serde_qs::to_string(&payload).unwrap()
            ))
            .header(
                "Authorization",
                gen_jwt_token(access_key()?, secret_key()?, &query_string),
            )
            .send()
            .await?;

        #[derive(Serialize, Deserialize, Debug)]
        struct Response {
            pub side: String,
            pub state: String,
            pub trades: Vec<Trade>,
            pub executed_volume: Decimal,
        }

        #[derive(Serialize, Deserialize, Debug)]
        struct Trade {
            pub funds: Decimal,
        }

        let status = response.status();
        let response = response.text().await?;
        tracing::debug!("Upbit::view_order() response: {}", response);
        if !status.is_success() {
            return Err(UpbitError::OrderFailed);
        }

        let response: Response = serde_json::from_str(&response).unwrap();
        let order_state = match response.state.as_str() {
            "wait" => OrderState::Wait,
            "cancel" | "done" => OrderState::Closed,
            _ => unreachable!(),
        };

        let qty = if response.side == "bid" {
            response.executed_volume
        } else {
            response.trades.iter().map(|t| t.funds).sum()
        };

        Ok(Order {
            state: order_state,
            executed_volume: qty,
        })
    }

    async fn wait_order(&self, order_token: &OrderToken) -> Result<Decimal, Self::Error> {
        loop {
            let order = self.view_order(order_token).await?;
            if order.state == OrderState::Closed {
                return Ok(order.executed_volume);
            }

            async_helpers::sleep(Duration::from_millis(250)).await;
        }
    }

    async fn withdraw(
        &self,
        currency: Currency,
        amount: Decimal,
        address1: &str,
        address2: Option<&str>,
        network: Option<&str>,
    ) -> Result<(), Self::Error> {
        let mut amount_rounded = amount.round_dp(6);
        if amount_rounded > amount {
            amount_rounded -= dec!(0.000001);
        }

        tracing::info!(
            "Upbit::withdraw({:?}, {}, {}, {:?}, {:?})",
            currency,
            amount_rounded,
            address1,
            address2,
            network
        );

        let mut message = json!({
            "currency": currency.to_string(),
            "amount": amount_rounded.to_string(),
            "address": address1,
            "transaction_type": "default",
        });

        if let Some(address) = address2 {
            message["secondary_address"] = json!(address);
        }

        if let Some(network) = network {
            message["net_type"] = json!(network);
        }

        println!("{:?}", message);

        let response = self
            .http_client
            .post("https://api.upbit.com/v1/withdraws/coin")
            .header(
                "Authorization",
                gen_jwt_token(
                    access_key()?,
                    secret_key()?,
                    &serde_qs::to_string(&message).unwrap(),
                ),
            )
            .body(serde_json::to_string(&message).unwrap())
            .send()
            .await;

        let Ok(response) = response else {
            return Err(UpbitError::WithdrawFailed);
        };

        let status = response.status();
        let response = response.text().await.unwrap();
        tracing::info!("Upbit::withdraw() response: {}", response);

        status
            .is_success()
            .then(|| ())
            .ok_or(UpbitError::WithdrawFailed)
    }

    async fn cancel_order(&self, order_token: &OrderToken) -> Result<Decimal, Self::Error> {
        unwrap_let!(OrderToken::String(order_token) = order_token);
        let payload = json!({
            "uuid": order_token,
        });

        let query_string = serde_qs::to_string(&payload).unwrap();
        let response = self
            .http_client
            .delete(&format!(
                "https://api.upbit.com/v1/order?{}",
                serde_qs::to_string(&payload).unwrap()
            ))
            .header(
                "Authorization",
                gen_jwt_token(access_key()?, secret_key()?, &query_string),
            )
            .send()
            .await?;

        #[derive(Serialize, Deserialize, Debug)]
        struct Response {
            pub executed_volume: Decimal,
        }

        let status = response.status();
        let response = response.text().await?;
        tracing::info!("Upbit::cancel_order() response: {}", response);
        if !status.is_success() {
            return Err(UpbitError::OrderFailed);
        }

        let response: Response = serde_json::from_str(&response)?;
        Ok(response.executed_volume)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum UpbitItem {
    Trade {
        code: String,
        trade_price: Decimal,
        trade_volume: Decimal,
        ask_bid: String,
        timestamp: u64,
    },
    Orderbook {
        code: String,
        orderbook_units: Vec<UpbitOrderbookUnit>,
    },
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct UpbitOrderbookUnit {
    pub ask_price: Decimal,
    pub ask_size: Decimal,
    pub bid_price: Decimal,
    pub bid_size: Decimal,
}

async fn spawn_ws_broadcaster(
    global_ctx: GlobalContext,
    broadcaster: broadcast::Broadcaster,
    subscriptions: Arc<RwLock<HashSet<(Currency, Currency)>>>,
) {
    let ws = Arc::new(Websocket::new());
    ws.connect("wss://api.upbit.com/websocket/v1").await;

    {
        let ws = ws.clone();
        global_ctx.spawn(async move {
            let mut subscribed = HashSet::new();
            loop {
                let mut is_modified = false;
                if let Some(subscriptions) = subscriptions.try_read() {
                    is_modified = subscribed != *subscriptions;
                    subscribed = subscriptions.clone();
                }

                if is_modified {
                    tracing::info!("Upbit::spawn_ws_broadcaster() subscribed: {:?}", subscribed);

                    let stringified = subscribed
                        .iter()
                        .map(|(c1, c2)| {
                            CurrencyPairDelimiterStringifier::<'-'>::stringify(*c2, *c1).unwrap()
                        })
                        .collect::<Vec<_>>();

                    ws.send(
                        &json!([
                            {"ticket":"rsader"},
                            {"type":"trade", "codes": stringified},
                            {"type":"orderbook", "codes": stringified},
                        ])
                        .to_string(),
                    )
                    .await;
                }

                async_helpers::sleep(Duration::from_secs(1)).await;
            }
        });
    }

    loop {
        let Some(message) = ws.recv().await else {
            ws.connect("wss://api.upbit.com/websocket/v1").await;
            async_helpers::sleep(Duration::from_secs(1)).await;
            continue;
        };

        let into_pair = |code: &str| -> (Currency, Currency) {
            let mut iter = code.split('-');
            let c1 = Currency::from_str(iter.next().unwrap()).unwrap();
            let c2 = Currency::from_str(iter.next().unwrap()).unwrap();
            (c2, c1)
        };

        let message: UpbitItem = serde_json::from_str(&message).unwrap();
        match message {
            UpbitItem::Trade {
                code,
                trade_price,
                trade_volume,
                ask_bid,
                timestamp,
            } => broadcaster.broadcast(
                Some(BroadcastFrom::Exchange("upbit")),
                crate::exchange::Trade {
                    pair: into_pair(&code),
                    price: trade_price,
                    qty: trade_volume,
                    timestamp,
                    is_bid: ask_bid == "BID",
                },
            ),
            UpbitItem::Orderbook {
                code,
                orderbook_units,
                ..
            } => {
                let pair = into_pair(&code);
                let bids = orderbook_units
                    .iter()
                    .map(|unit| Unit {
                        price: unit.bid_price,
                        amount: unit.bid_size,
                    })
                    .collect();
                let asks = orderbook_units
                    .iter()
                    .map(|unit| Unit {
                        price: unit.ask_price,
                        amount: unit.ask_size,
                    })
                    .collect();

                broadcaster.broadcast(
                    Some(BroadcastFrom::Exchange("upbit")),
                    Orderbook { pair, bids, asks },
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use rust_decimal::Decimal;
    use rust_decimal_macros::dec;

    use crate::{
        currency::Currency,
        exchange::{Exchange, Upbit},
    };

    #[ignore]
    #[tokio::test]
    async fn create_and_cancel_order() {
        let exchange = Upbit::new();

        let orderbook = exchange
            .orderbook((Currency::XRP, Currency::KRW), None)
            .await
            .unwrap();
        let lowest_bid_price = orderbook.bids.last().unwrap().price;

        println!("lowest bid price: {}", lowest_bid_price);

        let order_token = exchange
            .bid_limit(
                (Currency::XRP, Currency::KRW),
                lowest_bid_price,
                dec!(5500) / lowest_bid_price, // the minium order amount is 5000 krw
                None,
            )
            .await
            .unwrap();

        println!("{:?}", order_token);

        let executed_volume = exchange.cancel_order(&order_token).await.unwrap();
        assert_eq!(executed_volume, Decimal::new(0, 0));
    }

    #[cfg(test)]
    mod test {
        use crate::{
            currency::Currency,
            exchange::{Exchange, Upbit},
        };

        #[ignore]
        #[tokio::test]
        async fn balance() {
            let exchange = Upbit::new();
            let balance = exchange.balance(Currency::KRW, None).await.unwrap();
            println!("{:?}", balance);
        }
    }
}
