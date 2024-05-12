use std::{
    collections::HashSet,
    sync::{Arc, RwLock},
    time::Duration,
};

use num_traits::pow;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use unwrap_let::unwrap_let;

use crate::utils::Decimal;
use crate::{
    config::Config,
    currency::{Currency, CurrencyPairStringifier, NoDelimiterCurrencyPairStringifier},
    exchange::{Order, OrderState, Unit},
    utils::async_helpers,
    utils::http::{client, Client, Method},
};
use crate::{dec, utils::broadcaster::Subscription};

use super::{Balance, CandleSticks, Exchange, Market, OrderToken, Orderbook, RealtimeData};

#[derive(thiserror::Error, Debug)]
pub enum BinanceError {
    #[error("request error")]
    RequestError,

    #[error("http client error {0}")]
    HttpClientError(#[from] reqwest::Error),

    #[error("json error")]
    JsonError(#[from] serde_json::Error),

    #[error("order failed")]
    OrderFailed,

    #[error("order cancel failed")]
    OrderCancelFailed,

    #[error("view order failed")]
    ViewOrderFailed,

    #[error("withdraw failed")]
    WithdrawFailed,

    #[error("cofnig not found")]
    ConfigNotFound,
}

fn api_key() -> Result<&'static str, BinanceError> {
    Config::get()
        .binance
        .as_ref()
        .map(|c| c.api_key.as_str())
        .ok_or(BinanceError::ConfigNotFound)
}

fn secret_key() -> Result<&'static str, BinanceError> {
    Config::get()
        .binance
        .as_ref()
        .map(|c| c.secret_key.as_str())
        .ok_or(BinanceError::ConfigNotFound)
}

fn hmac_signature(secret_key: &str, message: &str) -> String {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;

    let mut mac = Hmac::<Sha256>::new_from_slice(secret_key.as_bytes()).unwrap();
    mac.update(message.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

pub struct Binance {
    subscriptions: Arc<RwLock<HashSet<(Currency, Currency)>>>,
    http_client: Client,
}

impl Binance {
    pub fn new() -> Self {
        Self {
            subscriptions: Arc::new(RwLock::new(HashSet::new())),
            http_client: client(),
        }
    }

    pub async fn get_orderbook(
        &self,
        pair: (Currency, Currency),
        market: Market,
    ) -> Result<String, BinanceError> {
        let pair = NoDelimiterCurrencyPairStringifier::stringify(pair.0, pair.1).unwrap();

        Ok(match market {
            Market::Spot => {
                self.http_client
                    .get("https://api.binance.com/api/v3/depth")
                    .query(&[("symbol", pair.as_str()), ("limit", "20")])
                    .send()
                    .await?
                    .text()
                    .await?
            }
            Market::Future => {
                self.http_client
                    .get("https://fapi.binance.com/fapi/v1/depth")
                    .query(&[("symbol", pair.as_str()), ("limit", "20")])
                    .send()
                    .await?
                    .text()
                    .await?
            }
        })
    }

    pub async fn get_balance(&self, currency: Currency, market: Market) -> Balance {
        let message = serde_json::json!({
            "timestamp": chrono::Utc::now().timestamp_millis(),
        });

        match market {
            Market::Spot => {
                #[derive(Deserialize)]
                struct Response {
                    balances: Vec<BinanceBalance>,
                }

                #[derive(Deserialize)]
                struct BinanceBalance {
                    asset: String,
                    free: Decimal,
                    locked: Decimal,
                }

                let response: Response = request_userdata_trade_kind(
                    Method::GET,
                    "https://api.binance.com/api/v3/account",
                    &self.http_client,
                    message,
                )
                .await
                .unwrap();

                let balance = response
                    .balances
                    .into_iter()
                    .find(|b| b.asset == currency.to_string())
                    .unwrap_or(BinanceBalance {
                        asset: currency.to_string(),
                        free: Decimal::ZERO,
                        locked: Decimal::ZERO,
                    });

                Balance {
                    available: balance.free,
                    locked: balance.locked,
                }
            }
            Market::Future => {
                let response: Vec<FutureBalance> = request_userdata_trade_kind(
                    Method::GET,
                    "https://fapi.binance.com/fapi/v2/balance",
                    &self.http_client,
                    message,
                )
                .await
                .unwrap();

                #[derive(Deserialize)]
                #[serde(rename_all = "camelCase")]
                struct FutureBalance {
                    asset: String,
                    balance: Decimal,
                    available_balance: Decimal,
                }

                let balance = response
                    .into_iter()
                    .find(|b| b.asset == currency.to_string())
                    .unwrap_or(FutureBalance {
                        asset: currency.to_string(),
                        balance: Decimal::ZERO,
                        available_balance: Decimal::ZERO,
                    });

                Balance {
                    available: balance.available_balance,
                    locked: balance.balance - balance.available_balance,
                }
            }
        }
    }

    pub async fn make_spot_order(
        &self,
        pair: (Currency, Currency),
        side: &str,
        order_type: &str,
        price: Option<Decimal>,
        amount: Decimal,
    ) -> Result<OrderToken, BinanceError> {
        let pair = NoDelimiterCurrencyPairStringifier::stringify(pair.0, pair.1).unwrap();

        let mut message = serde_json::json!({
            "symbol": pair,
            "side": side,
            "type": order_type,
            "timestamp": chrono::Utc::now().timestamp_millis(),
            "quantity": amount,
        });

        if order_type == "LIMIT" {
            message["timeInForce"] = serde_json::json!("GTC");
            message["price"] = serde_json::json!(price);
        }

        let query_string = serde_qs::to_string(&message).unwrap();
        let signature = hmac_signature(secret_key()?, &query_string);
        message["signature"] = serde_json::json!(signature);

        let response = self
            .http_client
            .post("https://api.binance.com/api/v3/order")
            .header("X-MBX-APIKEY", api_key()?)
            .body(serde_qs::to_string(&message).unwrap())
            .send()
            .await?;

        let status = response.status();
        let result = response.text().await?;

        tracing::info!(
            "Binance::spot_order({:?} {} {}) response: {}",
            price,
            amount,
            order_type,
            result
        );
        if !status.is_success() {
            return Err(BinanceError::OrderFailed);
        }

        let response: serde_json::Value = serde_json::from_str(&result).unwrap();
        Ok(OrderToken::Array(vec![
            response["orderId"].clone(),
            OrderToken::String("spot".into()),
            pair.into(),
        ]))
    }

    pub async fn make_future_order(
        &self,
        pair: (Currency, Currency),
        side: &str,
        order_type: &str,
        price: Option<Decimal>,
        amount: Decimal,
    ) -> Result<OrderToken, BinanceError> {
        let pair = NoDelimiterCurrencyPairStringifier::stringify(pair.0, pair.1).unwrap();

        let mut message = serde_json::json!({
            "symbol": pair,
            "side": side,
            "type": order_type,
            "timestamp": chrono::Utc::now().timestamp_millis(),
            "quantity": amount,
        });

        if order_type == "LIMIT" {
            message["timeInForce"] = serde_json::json!("GTC");
            message["price"] = serde_json::json!(price);
        }

        let response: serde_json::Value = request_userdata_trade_kind(
            Method::POST,
            "https://fapi.binance.com/fapi/v1/order",
            &self.http_client,
            message,
        )
        .await?;

        tracing::info!(
            "Binance::future_order({:?} {} {}) response: {}",
            price,
            amount,
            order_type,
            response
        );
        Ok(OrderToken::Array(vec![
            response["orderId"].clone(),
            OrderToken::String("future".into()),
            pair.into(),
        ]))
    }

    async fn view_spot_order(&self, order_token: OrderToken) -> Result<Order, BinanceError> {
        unwrap_let!(OrderToken::Array(order_token) = order_token);
        unwrap_let!(
            [OrderToken::Number(order_id), _, OrderToken::String(pair)] = order_token.as_slice()
        );

        let message = serde_json::json!({
            "timestamp": chrono::Utc::now().timestamp_millis(),
            "orderId": order_id,
            "symbol": pair,
        });

        let query_string = serde_qs::to_string(&message).unwrap();
        let signature = hmac_signature(secret_key()?, &query_string);

        let response_order = self
            .http_client
            .get("https://api.binance.com/api/v3/order")
            .header("X-MBX-APIKEY", api_key()?)
            .query(&message)
            .query(&[("signature", signature.clone())])
            .send()
            .await?;

        let status = response_order.status();
        let result = response_order.text().await?;

        tracing::debug!("Binance::view_order() response: {}", result);
        if !status.is_success() {
            return Err(BinanceError::ViewOrderFailed);
        }

        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Response {
            pub status: String,
            pub executed_qty: Decimal,
            pub cummulative_quote_qty: Decimal,
            pub side: String,
        }

        let response: Response = serde_json::from_str(&result).unwrap();
        let state = match response.status.as_str() {
            "FILLED" | "CANCELED" => OrderState::Closed,
            _ => OrderState::Wait,
        };

        let qty = if response.side == "BUY" {
            response.executed_qty
        } else {
            response.cummulative_quote_qty
        };

        // FIXME: Using fixed fee rate
        //        Please replace with the actual fee rate of the exchange which is `commisson`
        const FEE_RATE: Decimal = dec!(0.0012);
        Ok(Order {
            state,
            executed_volume: qty - (qty * FEE_RATE),
        })
    }

    async fn view_futures_order(&self, order_token: OrderToken) -> Result<Order, BinanceError> {
        unwrap_let!(OrderToken::Array(order_token) = order_token);
        unwrap_let!(
            [OrderToken::Number(order_id), _, OrderToken::String(pair)] = order_token.as_slice()
        );

        let message = serde_json::json!({
            "timestamp": chrono::Utc::now().timestamp_millis(),
            "orderId": order_id,
            "symbol": pair,
        });

        let query_string = serde_qs::to_string(&message).unwrap();
        let signature = hmac_signature(secret_key()?, &query_string);

        let response_order = self
            .http_client
            .get("https://fapi.binance.com/fapi/v1/order")
            .header("X-MBX-APIKEY", api_key()?)
            .query(&message)
            .query(&[("signature", signature.clone())])
            .send()
            .await?;

        let status = response_order.status();
        let result = response_order.text().await?;

        tracing::debug!("Binance::view_order() response: {}", result);
        if !status.is_success() {
            return Err(BinanceError::ViewOrderFailed);
        }

        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Response {
            pub status: String,
            pub executed_qty: Decimal,
        }

        let response: Response = serde_json::from_str(&result)?;
        let state = match response.status.as_str() {
            "FILLED" | "CANCELED" => OrderState::Closed,
            _ => OrderState::Wait,
        };

        Ok(Order {
            state,
            executed_volume: response.executed_qty,
        })
    }
}

impl Exchange for Binance {
    const NAME: &'static str = "binance";

    type Error = BinanceError;

    fn subscribe(
        &self,
        pair: (Currency, Currency),
        market: Option<Market>,
    ) -> Subscription<RealtimeData> {
        todo!()
    }

    async fn orderbook(
        &self,
        pair: (Currency, Currency),
        market: Option<Market>,
    ) -> Result<Orderbook, Self::Error> {
        let orderbook = self.get_orderbook(pair, market.unwrap_or_default()).await?;
        #[derive(Deserialize)]
        struct Depth {
            bids: Vec<(Decimal, Decimal)>,
            asks: Vec<(Decimal, Decimal)>,
        }

        let Depth { bids, asks } = serde_json::from_str(&orderbook)?;

        let asks = asks
            .into_iter()
            .map(|(price, amount)| Unit { price, amount })
            .collect();

        let bids = bids
            .into_iter()
            .map(|(price, amount)| Unit { price, amount })
            .collect();

        Ok(Orderbook { pair, bids, asks })
    }

    async fn candlesticks(
        &self,
        _pair: (Currency, Currency),
        _market: Option<Market>,
    ) -> Result<CandleSticks, Self::Error> {
        todo!()
    }

    async fn balance(
        &self,
        currency: Currency,
        market: Option<Market>,
    ) -> Result<Balance, Self::Error> {
        tracing::debug!("Binance::balance({:?})", currency);

        Ok(self.get_balance(currency, market.unwrap_or_default()).await)
    }

    async fn bid_limit(
        &self,
        pair: (Currency, Currency),
        price: Decimal,
        amount: Decimal,
        market: Option<Market>,
    ) -> Result<OrderToken, Self::Error> {
        tracing::info!("Binance::bid_limit({:?}, {}, {})", pair, price, amount);

        match market.unwrap_or_default() {
            Market::Spot => {
                self.make_spot_order(pair, "BUY", "LIMIT", Some(price), amount)
                    .await
            }
            Market::Future => todo!("future market is not supported yet"),
        }
    }

    async fn bid_market(
        &self,
        pair: (Currency, Currency),
        quote_qty: Decimal,
        market: Option<Market>,
    ) -> Result<OrderToken, Self::Error> {
        let orderbook = self.orderbook(pair, market).await?;

        Ok(match market.unwrap_or_default() {
            Market::Spot => {
                let qty = quote_qty / orderbook.asks[0].price;
                let qty = round_qty(pair.0, orderbook.asks[0].price, qty);

                self.make_spot_order(pair, "BUY", "MARKET", None, qty)
                    .await?
            }
            Market::Future => {
                let qty = round_qty(pair.0, orderbook.asks[0].price, quote_qty);
                self.make_future_order(pair, "BUY", "MARKET", None, qty)
                    .await?
            }
        })
    }

    async fn ask_limit(
        &self,
        pair: (Currency, Currency),
        price: Decimal,
        amount: Decimal,
        market: Option<Market>,
    ) -> Result<OrderToken, Self::Error> {
        tracing::info!("Binance::ask_limit({:?}, {}, {})", pair, price, amount);
        Ok(match market.unwrap_or_default() {
            Market::Spot => {
                self.make_spot_order(pair, "SELL", "Limit", Some(price), amount)
                    .await?
            }
            Market::Future => {
                self.make_future_order(pair, "SELL", "Limit", Some(price), amount)
                    .await?
            }
        })
    }

    async fn ask_market(
        &self,
        pair: (Currency, Currency),
        base_qty: Decimal,
        market: Option<Market>,
    ) -> Result<OrderToken, Self::Error> {
        let orderbook = self.orderbook(pair, market).await?;

        let qty = base_qty;
        let qty = round_qty(pair.0, orderbook.asks[0].price, qty);

        Ok(match market.unwrap_or_default() {
            Market::Spot => {
                self.make_spot_order(pair, "SELL", "MARKET", None, qty)
                    .await?
            }
            Market::Future => {
                self.make_future_order(pair, "SELL", "MARKET", None, qty)
                    .await?
            }
        })
    }

    async fn view_order(&self, order_token: &OrderToken) -> Result<Order, Self::Error> {
        let ot = order_token.clone();
        unwrap_let!(OrderToken::Array(order_token) = order_token);
        unwrap_let!(
            [OrderToken::Number(order_id), OrderToken::String(market), _] = order_token.as_slice()
        );

        tracing::debug!("Binance::view_order({}, {})", order_id, market);
        Ok(match market.as_str() {
            "spot" => self.view_spot_order(ot).await?,
            "future" => self.view_futures_order(ot).await?,
            _ => unreachable!("invalid market type"),
        })
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

    async fn cancel_order(&self, _order_token: &OrderToken) -> Result<Decimal, Self::Error> {
        Err(BinanceError::OrderCancelFailed)
    }

    async fn withdraw(
        &self,
        currency: Currency,
        mut amount: Decimal,
        address1: &str,
        address2: Option<&str>,
        network: Option<&str>,
    ) -> Result<(), Self::Error> {
        if currency != Currency::USDT {
            amount = round_qty_withdraw(
                self.orderbook((currency, Currency::USDT), None)
                    .await
                    .unwrap()
                    .asks[0]
                    .price,
                amount,
            );
        }

        tracing::info!(
            "Binance::withdraw({:?}, {}, {}, {:?}, {:?})",
            currency,
            amount,
            address1,
            address2,
            network
        );

        let currency = currency.to_string();
        let mut message = serde_json::json!({
            "coin": currency,
            "address": address1,
            "amount": amount,
            "timestamp": chrono::Utc::now().timestamp_millis(),
        });

        if let Some(address2) = address2 {
            message["addressTag"] = address2.into();
        }

        if let Some(network) = network {
            message["network"] = network.into();
        }

        let query_string = serde_qs::to_string(&message).unwrap();
        let signature = hmac_signature(secret_key()?, &query_string);
        message["signature"] = serde_json::json!(signature);

        let response = self
            .http_client
            .post(format!(
                "https://api.binance.com/sapi/v1/capital/withdraw/apply?{}",
                serde_qs::to_string(&message).unwrap()
            ))
            .header("X-MBX-APIKEY", api_key()?)
            .body(String::new())
            .send()
            .await;

        let Ok(response) = response else {
            return Err(BinanceError::WithdrawFailed);
        };

        let status = response.status();
        let result = response.text().await.unwrap();

        tracing::info!("Binance::withdraw() response: {}", result);
        status
            .is_success()
            .then(|| ())
            .ok_or(BinanceError::WithdrawFailed)
    }

    async fn set_leverage(
        &self,
        pair: Option<(Currency, Currency)>,
        value: u64,
    ) -> Result<(), Self::Error> {
        tracing::trace!("Binance::set_leverage({:?}, {})", pair, value);

        let pair = pair.expect("binance does not support global leverage setting");
        let pair = NoDelimiterCurrencyPairStringifier::stringify(pair.0, pair.1).unwrap();

        request_userdata_trade_kind::<serde_json::Value, _>(
            Method::POST,
            "https://fapi.binance.com/fapi/v1/leverage",
            &self.http_client,
            serde_json::json!({
                "symbol": pair,
                "leverage": value,
                "timestamp": chrono::Utc::now().timestamp_millis(),
            }),
        )
        .await?;

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BianaceItem {
    DepthUpdate {
        pair: (Currency, Currency),
        bids: Vec<(Decimal, Decimal)>,
        asks: Vec<(Decimal, Decimal)>,
    },
}

fn numeric_digits(mut n: Decimal) -> u32 {
    let mut digit = 0;
    while n > Decimal::ONE {
        n /= dec!(10);
        digit += 1;
    }

    digit
}

fn round_qty(currency: Currency, price: Decimal, target: Decimal) -> Decimal {
    let (rounded, decimal) = match currency {
        Currency::SOL | Currency::APT => (target.round_dp(2), 2),
        Currency::XRP => (target.round_dp(0), 0),
        _ => {
            if price < dec!(1) {
                let target = target.round_dp(0);
                (target, 0)
            } else {
                let mut round_digit = numeric_digits(price).saturating_sub(1);
                let mut rounded;
                loop {
                    rounded = target.round_dp(round_digit);
                    if (rounded / target).abs() > dec!(0.99) {
                        break;
                    }
                    round_digit += 1;
                }

                (rounded, round_digit)
            }
        }
    };

    // if rounded value is greater than target, then subtract 1 from the decimal place
    if rounded > target {
        rounded - dec!(1) / pow(dec!(10), decimal as usize)
    } else {
        rounded
    }
}

fn round_qty_withdraw(price: Decimal, target: Decimal) -> Decimal {
    let mut round_digit = numeric_digits(price).saturating_sub(1);
    let mut rounded;
    loop {
        rounded = target.round_dp(round_digit);

        if (dec!(1) - ((rounded / target).abs())).abs() < dec!(0.0001) {
            break;
        }
        round_digit += 1;
    }

    if rounded > target {
        rounded - dec!(1) / pow(dec!(10), round_digit as usize)
    } else {
        rounded
    }
}

#[cfg(test)]
mod tests {
    use crate::dec;

    use crate::{
        currency::Currency,
        exchange::{Binance, Exchange, Market},
    };

    #[test]
    fn round_qty_withdraw_test() {
        let price = dec!(8.158);
        let target = dec!(63.66);
        let rounded = super::round_qty_withdraw(price, target);
        assert_eq!(rounded, dec!(0.0000000001));
    }

    #[ignore]
    #[tokio::test]
    async fn spot_balance() {
        let binance = Binance::new();
        let balance = binance
            .balance(Currency::USDT, Some(Market::Spot))
            .await
            .unwrap();

        print!("{:?}", balance);
    }

    #[ignore]
    #[tokio::test]
    async fn future_balance() {
        let binance = Binance::new();
        let balance = binance
            .balance(Currency::USDT, Some(Market::Future))
            .await
            .unwrap();

        print!("{:?}", balance);
    }

    #[ignore]
    #[tokio::test]
    async fn set_leverage() {
        let binance = Binance::new();
        binance
            .set_leverage(Some((Currency::BTC, Currency::USDT)), 11)
            .await
            .unwrap();
    }
}

async fn request_userdata_trade_kind<'path, R, T>(
    method: Method,
    url: &'path str,
    client: &Client,
    message: T,
) -> Result<R, BinanceError>
where
    R: DeserializeOwned,
    T: Serialize,
{
    let query_string = serde_qs::to_string(&message).unwrap();
    let signature = hmac_signature(secret_key()?, &query_string);

    let response = client
        .request(method, url)
        .header("X-MBX-APIKEY", api_key()?)
        .query(&message)
        .query(&[("signature", signature.clone())])
        .body(String::new())
        .send()
        .await
        .unwrap();

    let status = response.status();
    let result = response.text().await.unwrap();

    if !status.is_success() {
        tracing::error!("Binance::<{}> response: {}", url, result);
        return Err(BinanceError::RequestError);
    } else {
        tracing::debug!("Binance::<{}> response: {}", url, result);
    }

    Ok(serde_json::from_str(&result).unwrap())
}
