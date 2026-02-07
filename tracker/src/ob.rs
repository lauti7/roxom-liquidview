use crate::{
    INSTRUMENT_AMOUNT_DECIMALS, ORDERBOOK_FETCH_INTERVAL_SECS,
    instrument_worker::InstrumentWorkerSender,
    prices::{BtcAmount, BtcPrice, InstrumentAmount, NanoBtcAmount},
};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, time::Duration};
use thiserror::Error;

#[derive(Debug, Deserialize, Serialize)]
struct RxmApiResponse<T> {
    pub data: T,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OrderbookSnapshotDTO {
    pub instrument_id: String,
    pub buys: Vec<PriceLevelDTO>,
    pub sells: Vec<PriceLevelDTO>,
    pub buy_sell_versus: BuyVsSellsDTO,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PriceLevelDTO {
    pub price: String,
    pub size: String,
    pub total: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BuyVsSellsDTO {
    pub buy: String,
    pub sell: String,
}

#[derive(Debug, Error)]
pub enum FetchOrderbookError {
    #[error("Failed to fetch orderbook snapshot: {0}")]
    FetchFailed(#[from] reqwest::Error),
    #[error("Failed to parse orderbook snapshot: {0}")]
    ParseFailed(String),
}

async fn fetch_orderbook_snapshot(
    symbol: &str,
) -> Result<OrderbookSnapshotDTO, FetchOrderbookError> {
    let url = format!(
        "https://api.roxom.com/v1/orderbook/snapshot?symbol={symbol}&instType=perpetual&levels=50&unit=sats",
    );
    let response = reqwest::get(&url).await?;
    let snapshot = response
        .json::<RxmApiResponse<OrderbookSnapshotDTO>>()
        .await
        .map_err(|e| FetchOrderbookError::ParseFailed(e.to_string()))?;
    Ok(snapshot.data)
}

pub type RawPriceInSats = i64;
pub type InstrumentAmountInDecimals = f64;
pub type TotalValueInSats = i64;

#[derive(Debug, Clone, Default)]
pub struct Orderbook {
    pub bids: BTreeMap<BtcPrice, InstrumentAmount>,
    pub asks: BTreeMap<BtcPrice, InstrumentAmount>,
}

impl Orderbook {
    fn from_snapshot(snapshot: OrderbookSnapshotDTO) -> Self {
        let mut bids = BTreeMap::new();
        for buy in snapshot.buys {
            let price = parse_as_sats(&buy.price);
            let size = parse_instrument_size_decimals(&buy.size);
            let base_size = InstrumentAmount::try_from_decimal(size, INSTRUMENT_AMOUNT_DECIMALS);
            bids.insert(BtcPrice::new(price), base_size);
        }

        let mut asks = BTreeMap::new();
        for sell in snapshot.sells {
            let price = parse_as_sats(&sell.price);
            let size = parse_instrument_size_decimals(&sell.size);
            let base_size = InstrumentAmount::try_from_decimal(size, INSTRUMENT_AMOUNT_DECIMALS);
            asks.insert(BtcPrice::new(price), base_size);
        }

        Self { bids, asks }
    }

    pub fn total_value(&self) -> BtcAmount {
        let mut total = BtcAmount::zero();
        for (&price, &size) in self.bids.iter() {
            let sum = price.base_to_quote(&size).to_sats_round_down();
            total = total + sum;
        }
        for (&price, &size) in self.asks.iter() {
            let sum = price.base_to_quote(&size).to_sats_round_down();
            total = total + sum;
        }

        total
    }

    pub fn mid_price(&mut self) -> Option<BtcPrice> {
        let bid_price = self.bids.last_entry().map(|p| *p.key());
        let ask_price = self.asks.first_entry().map(|p| *p.key());

        match (bid_price, ask_price) {
            (None, None) => None,
            (Some(bid_price), Some(ask_price)) => {
                tracing::debug!("bid price {bid_price:?}");
                tracing::debug!("ask price {ask_price:?}");
                Some(BtcPrice::new((bid_price.value() + ask_price.value()) / 2))
            }
            (Some(bid_price), None) => Some(bid_price),
            (None, Some(ask_price)) => Some(ask_price),
        }
    }

    pub fn try_exec(&self, order: MiniMarketOrder) -> Option<Execution> {
        let orders_to_match = match order.side {
            OrderSide::Buy => &self.asks,
            OrderSide::Sell => &self.bids,
        };

        if orders_to_match.is_empty() {
            return None;
        }

        let mut executions = Vec::new();

        let mut remaining_base_amount = order.base_amount.amount();

        let matching_prices: Vec<_> = match order.side {
            OrderSide::Buy => orders_to_match
                .range(..=order.limit_price)
                .map(|(price, _)| *price)
                .collect(),
            OrderSide::Sell => orders_to_match
                .range(order.limit_price..)
                .map(|(price, _)| *price)
                .rev() // Reverse to get highest prices first for sell orders
                .collect(),
        };

        for price in matching_prices {
            if remaining_base_amount <= 0 {
                break;
            }

            if let Some(size) = orders_to_match.get(&price) {
                let level_size = *size;
                let level_price = price;
                let matched_size = remaining_base_amount.min(level_size.amount());
                let matched_quote = level_price.base_to_quote(&InstrumentAmount::new(
                    matched_size,
                    order.base_amount.decimals,
                ));
                executions.push(Execution {
                    execution_price: level_price,
                    executed_base_amount: InstrumentAmount::new(matched_size, size.decimals),
                    executed_quote_amount: matched_quote,
                });
                remaining_base_amount -= matched_size;
            }
        }

        if executions.len() == 1 {
            return executions.first().cloned();
        }

        let mut execution = Execution {
            executed_base_amount: InstrumentAmount::zero(order.base_amount.decimals),
            executed_quote_amount: NanoBtcAmount::zero(),
            execution_price: BtcPrice::zero(),
        };

        for exec in executions {
            execution.executed_base_amount =
                execution.executed_base_amount + exec.executed_base_amount;
            execution.executed_quote_amount =
                execution.executed_quote_amount + exec.executed_quote_amount;
        }

        execution.execution_price = BtcPrice::from_base_to_nano_btc_ratio(
            &execution.executed_base_amount,
            &execution.executed_quote_amount,
        );

        Some(execution)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum OrderSide {
    Buy,
    Sell,
}

#[derive(Debug, Clone)]
pub struct MiniMarketOrder {
    pub base_amount: InstrumentAmount,
    pub side: OrderSide,
    pub limit_price: BtcPrice,
}

#[derive(Debug, Clone)]
pub struct Execution {
    pub execution_price: BtcPrice,
    pub executed_base_amount: InstrumentAmount,
    pub executed_quote_amount: NanoBtcAmount,
}

fn parse_as_sats(price_str: &str) -> RawPriceInSats {
    price_str.parse::<i64>().unwrap()
}

fn parse_instrument_size_decimals(size_str: &str) -> InstrumentAmountInDecimals {
    size_str.parse::<f64>().unwrap()
}

async fn fetch_orderbook(symbol: &str) -> Result<Orderbook, FetchOrderbookError> {
    let ob = fetch_orderbook_snapshot(symbol).await?;

    tracing::debug!("{symbol}'s orderbook snapshot {ob:?}");

    Ok(Orderbook::from_snapshot(ob))
}

pub async fn polling_orderbook_loop(symbol: &str, worker: InstrumentWorkerSender) {
    let mut interval = tokio::time::interval(Duration::from_secs(ORDERBOOK_FETCH_INTERVAL_SECS));
    loop {
        interval.tick().await;

        match fetch_orderbook(symbol).await {
            Ok(orderbook) => {
                tracing::debug!("new orderbook snapshot: {:?}", orderbook.total_value());
                let _ = worker.send(orderbook);
            }
            Err(err) => {
                tracing::error!("failed to fetch orderbook for {symbol}: {}", err);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_try_exec_mini_order() {
        let orderbook = Orderbook::from_snapshot(OrderbookSnapshotDTO {
            instrument_id: "GOLD-BTC".to_string(),
            buys: vec![PriceLevelDTO {
                price: "4858200".to_string(),
                size: "0.25".to_string(),
                total: "1214550".to_string(),
            }],
            sells: vec![PriceLevelDTO {
                price: "4865500".to_string(),
                size: "0.25".to_string(),
                total: "1214550".to_string(),
            }],
            buy_sell_versus: BuyVsSellsDTO {
                buy: "20117287".to_string(),
                sell: "46241810".to_string(),
            },
        });
        let order = MiniMarketOrder {
            base_amount: InstrumentAmount::new(15, 2),
            side: OrderSide::Buy,
            limit_price: BtcPrice::new(4865500),
        };

        let execs = orderbook.try_exec(order);
        let exec = execs.expect("must have executions");

        assert_eq!(exec.executed_base_amount.to_decimal(), 0.15);
        assert_eq!(
            exec.executed_quote_amount.to_sats_round_down().value(),
            729_825
        );
    }
}
