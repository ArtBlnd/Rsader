use std::collections::HashSet;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use unwrap_let::unwrap_let;

use crate::config::Config;
use crate::dec;
use crate::utils::broadcaster::{Broadcaster, Subscription};
use crate::utils::Decimal;
use crate::websocket::Websocket;
use crate::{
    currency::Currency,
    exchange::{Balance, Order, OrderState, Unit},
    utils::async_helpers,
    utils::http::{self, Client},
};

use super::{CandleSticks, Exchange, Market, OrderToken, Orderbook, RealtimeData, Ticker, Trade};

pub fn connect_key() -> Result<&'static str, BithumbError> {
    Config::get()
        .bithumb
        .as_ref()
        .map(|config| config.connect_key.as_str())
        .ok_or(BithumbError::ConfigNotFound)
}

pub fn secret_key() -> Result<&'static str, BithumbError> {
    Config::get()
        .bithumb
        .as_ref()
        .map(|config| config.secret_key.as_str())
        .ok_or(BithumbError::ConfigNotFound)
}

pub fn ko_name() -> Result<&'static str, BithumbError> {
    Config::get()
        .bithumb
        .as_ref()
        .map(|config| config.ko_name.as_str())
        .ok_or(BithumbError::ConfigNotFound)
}

pub fn en_name() -> Result<&'static str, BithumbError> {
    Config::get()
        .bithumb
        .as_ref()
        .map(|config| config.en_name.as_str())
        .ok_or(BithumbError::ConfigNotFound)
}

fn gen_api_sign(endpoint: &str, query_string: &str, nonce: u64, secret_key: &str) -> String {
    use base64::Engine;
    use hmac::{Hmac, Mac};
    use sha2::Sha512;

    let parameters = format!("{}\0{}\0{}", endpoint, query_string, nonce);

    let mut mac = Hmac::<Sha512>::new_from_slice(secret_key.as_bytes()).unwrap();
    mac.update(parameters.as_bytes());
    base64::engine::general_purpose::STANDARD.encode(hex::encode(mac.finalize().into_bytes()))
}

#[derive(thiserror::Error, Debug)]
pub enum BithumbError {
    #[error("http error: {0}")]
    HttpError(#[from] http::Error),

    #[error("serde_json error: {0}")]
    SerdeJsonError(#[from] serde_json::Error),

    #[error("bid/ask order failed")]
    OrderFailed,

    #[error("view order failed")]
    ViewOrderFailed,

    #[error("cancel order failed")]
    CancelOrderFailed,

    #[error("failed to get balance")]
    BalanceFailed,

    #[error("API key not found")]
    ConfigNotFound,

    #[error("withdraw failed")]
    WithdrawFailed,
}

pub struct Bithumb {
    broadcaster: RealtimeDataBroadcaster,
    http_client: Client,
}

impl Bithumb {
    pub fn new() -> Self {
        let broadcaster = RealtimeDataBroadcaster::new();
        broadcaster.spawn_and_broadcast();

        Self {
            broadcaster,
            http_client: http::client(),
        }
    }
}

impl Exchange for Bithumb {
    const NAME: &'static str = "bithumb";

    type Error = BithumbError;

    fn subscribe(
        &self,
        pair: (Currency, Currency),
        _market: Option<Market>,
    ) -> Subscription<RealtimeData> {
        self.broadcaster.subscribe(pair)
    }

    async fn orderbook(
        &self,
        pair: (Currency, Currency),
        _market: Option<Market>,
    ) -> Result<Orderbook, Self::Error> {
        let request = self
            .http_client
            .get(format!(
                "https://api.bithumb.com/public/orderbook/{:?}_{:?}",
                pair.0, pair.1,
            ))
            .send()
            .await?;

        let response = request.text().await?;
        #[derive(Deserialize)]
        struct Response1 {
            data: Response2,
        }

        #[derive(Deserialize)]
        struct Response2 {
            bids: Vec<Response3>,
            asks: Vec<Response3>,
        }

        #[derive(Deserialize)]
        struct Response3 {
            price: Decimal,
            quantity: Decimal,
        }

        let response: Response1 = serde_json::from_str(&response)?;
        let bids = response
            .data
            .bids
            .into_iter()
            .map(|item| Unit {
                price: item.price,
                amount: item.quantity,
            })
            .collect();

        let asks = response
            .data
            .asks
            .into_iter()
            .map(|item| Unit {
                price: item.price,
                amount: item.quantity,
            })
            .collect();

        Ok(Orderbook { pair, bids, asks })
    }

    async fn balance(
        &self,
        currency: Currency,
        _market: Option<Market>,
    ) -> Result<Balance, Self::Error> {
        let endpoint = "/info/balance";

        let mut payload = serde_json::json!({
            "endpoint": endpoint,

        });

        if currency != Currency::KRW {
            payload["currency"] = currency.to_string().into();
        }

        let payload = serde_qs::to_string(&payload).unwrap();
        let nonce = chrono::Utc::now().timestamp_millis() as u64;
        let api_sign = gen_api_sign(endpoint, &payload, nonce, secret_key()?);

        let response = self
            .http_client
            .post(format!("https://api.bithumb.com/info/balance"))
            .header("api-client-type", "0")
            .header("Api-Key", connect_key()?)
            .header("Api-Nonce", nonce.to_string())
            .header("Api-Sign", api_sign)
            .header("Accept", "application/json")
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(payload)
            .send()
            .await
            .unwrap();

        let status = response.status();
        let text = response.text().await.unwrap();

        tracing::debug!("Bithumb::total() response: {}", text);
        if !status.is_success() {
            return Err(BithumbError::BalanceFailed);
        }

        #[derive(Deserialize)]
        struct Resposne1 {
            pub data: serde_json::Value,
        }

        let text: Resposne1 = serde_json::from_str(&text)?;

        let in_use;
        let available;
        if currency == Currency::KRW {
            in_use = serde_json::Value::String("0".to_string());
            available = text.data.get("total_krw").unwrap();
        } else {
            in_use = text
                .data
                .get(format!("in_use_{}", currency.to_string().to_lowercase()))
                .unwrap()
                .clone();
            available = text
                .data
                .get(format!("available_{}", currency.to_string().to_lowercase()))
                .unwrap();
        };

        unwrap_let!(serde_json::Value::String(in_use) = in_use);
        unwrap_let!(serde_json::Value::String(available) = available);

        let in_use = Decimal::from_str(&in_use).unwrap();
        let available = Decimal::from_str(&available).unwrap();

        Ok(Balance {
            locked: in_use,
            available,
        })
    }

    async fn bid_limit(
        &self,
        pair: (Currency, Currency),
        price: Decimal,
        amount: Decimal,
        _market: Option<Market>,
    ) -> Result<OrderToken, Self::Error> {
        tracing::info!("Bithumb::bid_limit({:?})", pair);
        let order_currency = pair.0.to_string();
        let payment_currency = pair.1.to_string();

        let endpoint = "/trade/place";
        let payload = serde_qs::to_string(&serde_json::json!({
            "endpoint": endpoint,
            "order_currency": order_currency,
            "payment_currency": payment_currency,
            "units": amount,
            "price": price,
            "type": "bid",
        }))
        .unwrap();

        let nonce = chrono::Utc::now().timestamp_millis() as u64;
        let api_sign = gen_api_sign(endpoint, &payload, nonce, secret_key()?);

        let response = self
            .http_client
            .post(format!("https://api.bithumb.com/trade/place"))
            .header("api-client-type", "0")
            .header("Api-Key", connect_key()?)
            .header("Api-Nonce", nonce.to_string())
            .header("Api-Sign", api_sign)
            .header("Accept", "application/json")
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(payload)
            .send()
            .await?;

        let status = response.status();
        let text = response.text().await?;

        tracing::info!("Bithumb::bid_limit() response: {}", text);
        let response: serde_json::Value = serde_json::from_str(&text).unwrap();
        if !status.is_success() || response.get("status").unwrap() != "0000" {
            return Err(BithumbError::OrderFailed);
        }

        Ok(OrderToken::Array(vec![
            response
                .get("order_id")
                .ok_or(BithumbError::OrderFailed)?
                .clone(),
            OrderToken::String(order_currency),
            OrderToken::String(payment_currency),
        ]))
    }

    async fn bid_market(
        &self,
        pair: (Currency, Currency),
        quote_qty: Decimal,
        market: Option<Market>,
    ) -> Result<OrderToken, Self::Error> {
        tracing::info!("Bithumb::bid_market({:?}, {})", pair, quote_qty);
        let orderbook = self.orderbook(pair, market).await?;
        let ask = orderbook.asks.first().unwrap();

        let order_currency = pair.0.to_string();
        let payment_currency = pair.1.to_string();

        let endpoint = "/trade/market_buy";

        let payload = serde_qs::to_string(&serde_json::json!({
            "endpoint": endpoint,
            "units": (quote_qty / ask.price).round_dp(4) - dec!(0.0001),
            "order_currency": order_currency,
            "payment_currency": payment_currency,
        }))
        .unwrap();

        let nonce = chrono::Utc::now().timestamp_millis() as u64;
        let api_sign = gen_api_sign(endpoint, &payload, nonce, secret_key()?);

        let response = self
            .http_client
            .post(format!("https://api.bithumb.com/trade/market_buy"))
            .header("api-client-type", "0")
            .header("Api-Key", connect_key()?)
            .header("Api-Nonce", nonce.to_string())
            .header("Api-Sign", api_sign)
            .header("Accept", "application/json")
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(payload)
            .send()
            .await
            .unwrap();

        let status = response.status();
        let text = response.text().await.unwrap();

        tracing::info!("Bithumb::bid_market() response: {}", text);
        let response: serde_json::Value = serde_json::from_str(&text).unwrap();
        if !status.is_success() || response.get("status").unwrap() != "0000" {
            return Err(BithumbError::OrderFailed);
        }

        async_helpers::sleep(std::time::Duration::from_millis(250)).await;
        Ok(OrderToken::Array(vec![
            response
                .get("order_id")
                .ok_or(BithumbError::OrderFailed)?
                .clone(),
            OrderToken::String(order_currency),
            OrderToken::String(payment_currency),
        ]))
    }

    async fn ask_limit(
        &self,
        pair: (Currency, Currency),
        price: Decimal,
        amount: Decimal,
        _market: Option<Market>,
    ) -> Result<OrderToken, Self::Error> {
        tracing::info!("Bithumb::ask_limit({:?})", pair);
        let order_currency = pair.0.to_string();
        let payment_currency = pair.1.to_string();

        let endpoint = "/trade/place";

        let payload = serde_qs::to_string(&serde_json::json!({
            "endpoint": endpoint,
            "order_currency": order_currency,
            "payment_currency": payment_currency,
            "units": amount,
            "price": price,
            "type": "ask",
        }))
        .unwrap();

        let nonce = chrono::Utc::now().timestamp_millis() as u64;
        let api_sign = gen_api_sign(endpoint, &payload, nonce, secret_key()?);

        let response = self
            .http_client
            .post(format!("https://api.bithumb.com/trade/place"))
            .header("api-client-type", "0")
            .header("Api-Key", connect_key()?)
            .header("Api-Nonce", nonce.to_string())
            .header("Api-Sign", api_sign)
            .header("Accept", "application/json")
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(payload)
            .send()
            .await
            .unwrap();

        let status = response.status();
        let text = response.text().await.unwrap();

        tracing::info!("Bithumb::ask_limit() response: {}", text);
        let response: serde_json::Value = serde_json::from_str(&text).unwrap();
        if !status.is_success() || response.get("status").unwrap() != "0000" {
            return Err(BithumbError::OrderFailed);
        }

        Ok(OrderToken::Array(vec![
            response
                .get("order_id")
                .ok_or(BithumbError::OrderFailed)?
                .clone(),
            OrderToken::String(order_currency),
            OrderToken::String(payment_currency),
        ]))
    }

    async fn ask_market(
        &self,
        pair: (Currency, Currency),
        base_qty: Decimal,
        _market: Option<Market>,
    ) -> Result<OrderToken, Self::Error> {
        tracing::info!("Bithumb::ask_market({:?})", pair);
        let order_currency = pair.0.to_string();
        let payment_currency = pair.1.to_string();

        let endpoint = "/trade/market_sell";

        let payload = serde_qs::to_string(&serde_json::json!({
            "endpoint": endpoint,
            "units": base_qty,
            "order_currency": order_currency,
            "payment_currency": payment_currency,
        }))
        .unwrap();

        let nonce = chrono::Utc::now().timestamp_millis() as u64;
        let api_sign = gen_api_sign(endpoint, &payload, nonce, secret_key()?);

        let response = self
            .http_client
            .post(format!("https://api.bithumb.com/trade/market_sell"))
            .header("api-client-type", "0")
            .header("Api-Key", connect_key()?)
            .header("Api-Nonce", nonce.to_string())
            .header("Api-Sign", api_sign)
            .header("Accept", "application/json")
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(payload)
            .send()
            .await
            .unwrap();

        let status = response.status();
        let text = response.text().await.unwrap();

        tracing::info!("Bithumb::ask_market() response: {}", text);
        let response: serde_json::Value = serde_json::from_str(&text).unwrap();
        if !status.is_success() || response.get("status").unwrap() != "0000" {
            return Err(BithumbError::OrderFailed);
        }

        async_helpers::sleep(std::time::Duration::from_millis(250)).await;
        Ok(OrderToken::Array(vec![
            response
                .get("order_id")
                .ok_or(BithumbError::OrderFailed)?
                .clone(),
            OrderToken::String(order_currency),
            OrderToken::String(payment_currency),
        ]))
    }

    async fn view_order(&self, order_token: &OrderToken) -> Result<Order, Self::Error> {
        unwrap_let!(OrderToken::Array(order_info) = order_token);
        unwrap_let!([order_id, order_currency, payment_currency] = order_info.as_slice());

        let endpoint = "/info/order_detail";
        let payload = serde_qs::to_string(&serde_json::json!({
            "endpoint": endpoint,
            "order_id": order_id,
            "order_currency": order_currency,
            "payment_currency": payment_currency,
        }))
        .unwrap();

        let nonce = chrono::Utc::now().timestamp_millis() as u64;
        let api_sign = gen_api_sign(endpoint, &payload, nonce, secret_key()?);

        let response = self
            .http_client
            .post(format!("https://api.bithumb.com/info/order_detail"))
            .header("api-client-type", "0")
            .header("Api-Key", connect_key()?)
            .header("Api-Nonce", nonce.to_string())
            .header("Api-Sign", api_sign)
            .header("Accept", "application/json")
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(payload)
            .send()
            .await?;

        let status = response.status();
        let text = response.text().await?;

        tracing::debug!("Bithumb::view_order() response: {}", text);
        if !status.is_success() {
            return Err(BithumbError::ViewOrderFailed);
        }

        #[derive(Deserialize)]
        struct Resposne1 {
            pub data: Response2,
        }

        #[derive(Deserialize)]
        struct Response2 {
            pub order_status: String,
            pub contract: Vec<Contract>,
        }

        #[derive(Deserialize)]
        struct Contract {
            pub units: Decimal,
        }

        let response: Resposne1 = serde_json::from_str(&text)?;
        let order = Order {
            executed_volume: response.data.contract.iter().map(|c| c.units).sum(),
            state: match response.data.order_status.as_str() {
                "Completed" | "Cancel" => OrderState::Closed,
                _ => OrderState::Wait,
            },
        };

        Ok(order)
    }

    async fn wait_order(&self, _order_token: &OrderToken) -> Result<Decimal, Self::Error> {
        loop {
            let order = self.view_order(_order_token).await?;
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
        let endpoint = "/trade/btc_withdrawal";
        let mut query_string = serde_json::json!({
            "endpoint": endpoint,
            "currency": currency.to_string(),
            "units": amount,
            "address": address1,
            "cust_type_cd": "01",
            "exchange_name": "metamask",
            "ko_name": ko_name()?,
            "en_name": en_name()?,
        });

        if let Some(address2) = address2 {
            query_string["destination"] = serde_json::Value::String(address2.to_string());
        }

        if let Some(network) = network {
            query_string["net_type"] = serde_json::Value::String(network.to_string());
        }

        let payload = serde_qs::to_string(&query_string).unwrap();
        let nonce = chrono::Utc::now().timestamp_millis() as u64;
        let api_sign = gen_api_sign(endpoint, &payload, nonce, secret_key()?);

        let response = self
            .http_client
            .post(format!("https://api.bithumb.com/trade/btc_withdrawal"))
            .header("api-client-type", "0")
            .header("Api-Key", connect_key()?)
            .header("Api-Nonce", nonce.to_string())
            .header("Api-Sign", api_sign)
            .header("Accept", "application/json")
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(payload)
            .send()
            .await;

        let Ok(response) = response else {
            return Err(BithumbError::WithdrawFailed);
        };

        let status = response.status();
        let text = response.text().await.unwrap();
        tracing::info!("Bithumb::withdraw() response: {}", text);

        status
            .is_success()
            .then(|| ())
            .ok_or(BithumbError::WithdrawFailed)
    }

    async fn candlesticks(
        &self,
        pair: (Currency, Currency),
        _market: Option<Market>,
    ) -> Result<CandleSticks, Self::Error> {
        use num_traits::ToPrimitive;

        let url = format!(
            "https://api.bithumb.com/public/candlestick/{:?}_{:?}/10m",
            pair.0, pair.1
        );

        let response = self.http_client.get(url).send().await?;
        let text = response.text().await?;

        #[derive(Deserialize)]
        struct Response {
            data: Vec<[Decimal; 6]>,
        }

        let response: Response = serde_json::from_str(&text).unwrap();
        let tickers = response
            .data
            .into_iter()
            .map(|item| Ticker {
                timestamp: item[0].to_u64().unwrap(),
                open: item[1],
                close: item[2],
                high: item[3],
                low: item[4],
            })
            .collect();

        Ok(CandleSticks { pair, tickers })
    }

    async fn cancel_order(&self, order_token: &OrderToken) -> Result<Decimal, Self::Error> {
        unwrap_let!(OrderToken::Array(order_info) = order_token);
        unwrap_let!([order_id, order_currency, payment_currency] = order_info.as_slice());

        let endpoint = "/trade/cancel";
        let payload = serde_qs::to_string(&serde_json::json!({
            "endpoint": endpoint,
            "order_id": order_id,
            "order_currency": order_currency,
            "payment_currency": payment_currency,
        }))
        .unwrap();

        let nonce = chrono::Utc::now().timestamp_millis() as u64;
        let api_sign = gen_api_sign(endpoint, &payload, nonce, secret_key()?);

        let response = self
            .http_client
            .post(format!("https://api.bithumb.com/trade/cancel"))
            .header("api-client-type", "0")
            .header("Api-Key", connect_key()?)
            .header("Api-Nonce", nonce.to_string())
            .header("Api-Sign", api_sign)
            .header("Accept", "application/json")
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(payload)
            .send()
            .await?;

        let status = response.status();
        let text = response.text().await.unwrap();

        #[derive(Deserialize)]
        pub struct Message {
            status: String,
        }

        let message: Message = serde_json::from_str(&text)?;

        tracing::info!("Bithumb::cancel_order() response: {}", text);
        if !status.is_success() || message.status != "0000" {
            return Err(BithumbError::CancelOrderFailed);
        }

        Ok(Decimal::ZERO)
    }

    async fn set_leverage(
        &self,
        _pair: Option<(Currency, Currency)>,
        _value: u64,
    ) -> Result<(), Self::Error> {
        unimplemented!()
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(tag = "type", content = "content", rename_all = "lowercase")]
pub enum BithumbItem {
    OrderbookSnapshot {
        symbol: String,
        asks: Vec<(Decimal, Decimal)>,
        bids: Vec<(Decimal, Decimal)>,
    },
    Transaction {
        symbol: String,
        buy_sell_gb: String,
        cont_amt: Decimal,
        cont_price: Decimal,
    },
}

#[derive(Clone)]
pub struct RealtimeDataBroadcaster {
    subscribed: Arc<Mutex<HashSet<(Currency, Currency)>>>,
    broadcaster: Broadcaster<RealtimeData>,

    ws1: Websocket,
    ws2: Websocket,
}

impl RealtimeDataBroadcaster {
    pub fn new() -> Self {
        Self {
            subscribed: Arc::new(Mutex::new(HashSet::new())),
            broadcaster: Broadcaster::new(),

            ws1: Websocket::new("wss://pubwss.bithumb.com/pub/ws"),
            ws2: Websocket::new("wss://pubwss.bithumb.com/pub/ws"),
        }
    }

    fn spawn_and_broadcast(&self) {
        {
            let broadcaster = self.clone();
            async_helpers::spawn(async move {
                loop {
                    broadcaster.recv_orderbook_data_and_broadcast().await;
                }
            });
        }
        {
            let broadcaster = self.clone();
            async_helpers::spawn(async move {
                loop {
                    broadcaster.recv_transaction_data_and_broadcast().await;
                }
            });
        }
    }

    fn subscribe(&self, pair: (Currency, Currency)) -> Subscription<RealtimeData> {
        let mut subscribed = self.subscribed.lock().unwrap();
        if subscribed.insert(pair) {
            let pairs: Vec<_> = subscribed
                .iter()
                .map(|(base, quote)| format!("{:?}_{:?}", base, quote))
                .collect();

            self.ws1.send(
                &serde_json::json!({
                    "type": "orderbooksnapshot",
                    "symbols": pairs,
                })
                .to_string(),
            );

            self.ws2.send(
                &serde_json::json!({
                    "type": "transaction",
                    "symbols": pairs,
                })
                .to_string(),
            );
        }

        self.broadcaster.subscribe()
    }

    async fn recv_orderbook_data_and_broadcast(&self) {
        let data = self.ws1.recv().await.unwrap();
        let Ok(data) = serde_json::from_str::<BithumbItem>(&data) else {
            return;
        };

        let BithumbItem::OrderbookSnapshot { symbol, asks, bids } = data else {
            unreachable!()
        };

        let pair = {
            let mut iter = symbol.split('_');
            (
                Currency::from_str(iter.next().unwrap()).unwrap(),
                Currency::from_str(iter.next().unwrap()).unwrap(),
            )
        };

        let data = RealtimeData::Orderbook(Orderbook {
            pair,
            bids: bids
                .into_iter()
                .map(|(price, amount)| Unit { price, amount })
                .collect(),
            asks: asks
                .into_iter()
                .map(|(price, amount)| Unit { price, amount })
                .collect(),
        });
        self.broadcaster.broadcast(data);
    }

    async fn recv_transaction_data_and_broadcast(&self) {
        let data = self.ws2.recv().await.unwrap();
        let Ok(data) = serde_json::from_str::<BithumbItem>(&data) else {
            return;
        };

        let BithumbItem::Transaction {
            symbol,
            buy_sell_gb,
            cont_amt,
            cont_price,
        } = data
        else {
            unreachable!()
        };

        let pair = {
            let mut iter = symbol.split('_');
            (
                Currency::from_str(iter.next().unwrap()).unwrap(),
                Currency::from_str(iter.next().unwrap()).unwrap(),
            )
        };

        let data = RealtimeData::Trade(Trade {
            pair,
            timestamp: chrono::Utc::now().timestamp_millis(),
            price: cont_price,
            amount: cont_amt,
            is_bid: buy_sell_gb == "B",
        });
        self.broadcaster.broadcast(data);
    }
}

#[cfg(test)]
mod test {
    use crate::{
        currency::Currency,
        exchange::{Bithumb, Exchange},
    };

    #[ignore]
    #[tokio::test]
    async fn balance() {
        let exchange = Bithumb::new();
        let balance = exchange.balance(Currency::USDT, None).await.unwrap();

        println!("{:?}", balance);
    }
}
