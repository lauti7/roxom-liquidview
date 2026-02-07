use crate::{
    INSTRUMENT_BTC_PRICE_TICK_SIZE, MAX_SLIPPAGE_BY_INSTRUENT_SPEC, ORDERS_VALUES_IN_SATS,
    QUANTITY_TICK_SIZE,
    ob::{self, MiniMarketOrder, OrderSide, Orderbook},
    pg::DBWriterWorker,
    prices::{BtcAmount, BtcPrice, InstrumentAmount},
};
use tokio::{
    sync::mpsc::{UnboundedReceiver, UnboundedSender},
    task::JoinHandle,
};
use tokio_util::sync::CancellationToken;

pub type InstrumentWorkerSender = UnboundedSender<Orderbook>;

pub struct InstrumentWorker {
    symbol: String,
    rx: UnboundedReceiver<Orderbook>,
    _poll_ob_task: JoinHandle<()>,
    cancelation_token: CancellationToken,
    db_writer: DBWriterWorker,
}

impl InstrumentWorker {
    pub fn new(
        symbol: &str,
        db_writer: DBWriterWorker,
        cancelation_token: CancellationToken,
    ) -> Self {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();

        let symbol = symbol.to_string();
        let cloned_symbol = symbol.clone();
        let handle = tokio::spawn(async move {
            ob::polling_orderbook_loop(&cloned_symbol, tx).await;
        });

        Self {
            symbol,
            db_writer,
            rx,
            _poll_ob_task: handle,
            cancelation_token,
        }
    }

    pub async fn run(mut self) {
        loop {
            tokio::select! {
                _ = self.cancelation_token.cancelled() => {
                    tracing::info!("SymbolWorker for symbol {} cancelled", self.symbol);
                    break;
                }
                msg = self.rx.recv() => {
                    match msg {
                        Some(orderbook) => {
                            match try_orderbook_executions(&self.symbol, orderbook) {
                                Some(execution_costs) => {
                                    let _ = self.db_writer.send_events(&self.symbol, execution_costs);
                                }
                                None => {
                                    tracing::warn!("Symbol's {} returned None, not working", self.symbol);
                                }
                            }
                        }
                        None => {
                            tracing::info!("SymbolWorker for symbol {} closed", self.symbol);
                            break;
                        }
                    }
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct ExecutionCostEvent {
    pub order_value: BtcAmount,
    pub bps_over_mid_price: f64,
    pub mid_price: BtcPrice,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

type BpsOverMid = f64;

fn try_orderbook_executions(
    symbol: &str,
    mut orderbook: Orderbook,
) -> Option<Vec<ExecutionCostEvent>> {
    if let Some(mid_price) = orderbook.mid_price() {
        let mut execs = Vec::with_capacity(ORDERS_VALUES_IN_SATS.len());
        for order_value in ORDERS_VALUES_IN_SATS {
            let buy_bps = simulate_buy(symbol, mid_price, &mut orderbook, order_value);
            let sell_bps = simulate_sell(symbol, mid_price, &mut orderbook, order_value);

            let bps = match (buy_bps, sell_bps) {
                (Some(buy_bps), Some(sell_bps)) => (buy_bps + sell_bps) / 2.0,
                (Some(buy_bps), None) => buy_bps,
                (None, Some(sell_bps)) => sell_bps,
                _ => 0.0,
            };
            execs.push(ExecutionCostEvent {
                order_value: BtcAmount::new(order_value),
                bps_over_mid_price: bps,
                mid_price,
                timestamp: chrono::Utc::now(),
            });
        }

        return Some(execs);
    }

    tracing::warn!("{symbol} orderbooks doesn't havea mid price");

    None
}

fn simulate_buy(
    symbol: &str,
    mid_price: BtcPrice,
    orderbook: &mut Orderbook,
    order_value: i64,
) -> Option<BpsOverMid> {
    let with_slippage_limit_price =
        (mid_price.value() as f64 * (1.0 + MAX_SLIPPAGE_BY_INSTRUENT_SPEC)) as i64;
    let price = BtcPrice::new(with_slippage_limit_price)
        .next_multiple(&BtcPrice::new(INSTRUMENT_BTC_PRICE_TICK_SIZE));

    let order_value = BtcAmount::new(order_value);
    let base_amount =
        mid_price.quote_to_base(order_value, InstrumentAmount::new(QUANTITY_TICK_SIZE, 2));

    match orderbook.try_exec(MiniMarketOrder {
        base_amount,
        side: OrderSide::Buy,
        limit_price: price,
    }) {
        Some(execution) => {
            let variation =
                get_variation_percent(mid_price.value(), execution.execution_price.value());
            let bps_over_mid = (variation * 100.00).round();
            tracing::info!(
                side = "buy",
                order_value = order_value.value(),
                execution_price = %execution.execution_price.value(),
                limit_price = %price.value(),
                mid_price = %mid_price.value(),
                bps_over_mid = bps_over_mid,
                base_amount = ?base_amount.amount()
            );

            Some(bps_over_mid)
        }
        None => {
            tracing::warn!(
                "{symbol} orderbook's Buy execution None for order value {order_value:?}",
            );

            None
        }
    }
}

fn simulate_sell(
    symbol: &str,
    mid_price: BtcPrice,
    orderbook: &mut Orderbook,
    order_value: i64,
) -> Option<BpsOverMid> {
    let order_value = BtcAmount::new(order_value);
    let with_slippage_limit_price =
        (mid_price.value() as f64 * (1.0 - MAX_SLIPPAGE_BY_INSTRUENT_SPEC)) as i64;
    let price = BtcPrice::new(with_slippage_limit_price)
        .next_multiple(&BtcPrice::new(INSTRUMENT_BTC_PRICE_TICK_SIZE));

    let base_amount =
        mid_price.quote_to_base(order_value, InstrumentAmount::new(QUANTITY_TICK_SIZE, 2));

    match orderbook.try_exec(MiniMarketOrder {
        base_amount,
        side: OrderSide::Sell,
        limit_price: price,
    }) {
        Some(execution) => {
            let variation =
                get_variation_percent(mid_price.value(), execution.execution_price.value());
            let bps_over_mid = (variation * 100.00).round();
            tracing::info!(
                side = "sell",
                order_value = order_value.value(),
                execution_price = %execution.execution_price.value(),
                limit_price = %price.value(),
                mid_price = %mid_price.value(),
                bps_over_mid = bps_over_mid,
                base_amount = ?base_amount.amount()
            );

            Some(bps_over_mid)
        }
        None => {
            tracing::warn!(
                "{symbol} orderbook's Sell execution None for order value {order_value:?}",
            );
            None
        }
    }
}

fn get_variation_percent(initial: i64, last: i64) -> f64 {
    let a = ((last - initial) as f64 / initial as f64) * 100_f64;
    let a = (a * 100.00) as i64;
    (a as f64 / 100.00).abs()
}
