pub mod api;
pub mod http;
pub mod instrument_worker;
pub mod liquidity_service;
pub mod ob;
pub mod pg;
pub mod prices;

pub mod constants {
    pub const TRACKED_SYMBOLS: [&str; 1] = ["GOLD-BTC"];
    pub const ORDERBOOK_FETCH_INTERVAL_SECS: u64 = 30;
    pub const ORDERS_VALUES_IN_SATS: [i64; 5] =
        [250_000, 500_000, 1_000_000, 10_000_000, 50_000_000];
    // TODO: fetch instruments settings and use that.
    pub const MAX_SLIPPAGE_BY_INSTRUENT_SPEC: f64 = 0.02;
    // TODO: fetch instruments settings and use that.
    pub const QUANTITY_TICK_SIZE: i64 = 1; // scaled by 10^2
    // TODO: fetch instruments settings and use that.
    pub const INSTRUMENT_AMOUNT_DECIMALS: u8 = 2;
    // TODO: fetch instruments settings and use that.
    pub const INSTRUMENT_BTC_PRICE_TICK_SIZE: i64 = 100;
    // TODO: improve this
    pub const STANDARD_TAKER_FEE_DECIMALS: f64 = 0.00096;
    pub const STANDARD_TAKER_FEE_PCT: f64 = 0.096;
    pub const STANDARD_TAKER_FEE_BPS: f64 = 9.6;
}

pub use constants::*;
