use std::ops::{Add, Sub};

#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub struct InstrumentAmount {
    amount: i64,
    pub decimals: u8,
}

impl InstrumentAmount {
    pub fn new(amount: i64, decimals: u8) -> Self {
        Self { amount, decimals }
    }

    pub fn with_amount(mut self, amount: i64) -> Self {
        self.amount = amount;
        self
    }

    pub fn abs(&self) -> Self {
        self.with_amount(self.amount.abs())
    }

    pub fn into_zero(self) -> Self {
        self.with_amount(0)
    }

    pub fn zero(decimals: u8) -> Self {
        Self {
            amount: 0,
            decimals,
        }
    }

    pub fn amount(&self) -> i64 {
        self.amount
    }

    pub fn round_down(&self, min_tick: &InstrumentAmount) -> Self {
        let n = (self.amount / min_tick.amount).max(1);
        Self::new(n * min_tick.amount, self.decimals)
    }

    pub fn try_from_decimal(value: f64, decimals: u8) -> Self {
        if value < i64::MIN as f64 || value > i64::MAX as f64 {
            panic!("Value '{value}' is out of range for i64")
        }

        let scaled = value * 10_f64.powi(decimals as i32);

        if scaled < i64::MIN as f64 || scaled > i64::MAX as f64 {
            panic!("scaled value '{scaled}' is out of range for i64")
        }

        // Add 0.5 for proper rounding instead of truncation
        let amount = if scaled >= 0.0 {
            (scaled + 0.5) as i64
        } else {
            (scaled - 0.5) as i64
        };

        Self { amount, decimals }
    }

    pub fn to_decimal(&self) -> f64 {
        self.amount as f64 / 10_f64.powi(self.decimals as i32)
    }
}

impl Add<InstrumentAmount> for InstrumentAmount {
    type Output = InstrumentAmount;

    fn add(self, rhs: InstrumentAmount) -> Self::Output {
        InstrumentAmount::new(self.amount + rhs.amount, self.decimals)
    }
}

impl Sub<InstrumentAmount> for InstrumentAmount {
    type Output = InstrumentAmount;

    fn sub(self, rhs: InstrumentAmount) -> Self::Output {
        InstrumentAmount::new(self.amount - rhs.amount, self.decimals)
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Default, Hash)]
pub struct BtcAmount(i64);

impl BtcAmount {
    pub const BTC_DECIMALS: i32 = 8;

    pub fn zero() -> Self {
        Self(0)
    }

    pub fn new(value: i64) -> Self {
        Self(value)
    }

    pub fn from_decimal(value: f64) -> Self {
        let scaled = value * 10_f64.powi(8);
        // Add 0.5 for proper rounding instead of truncation
        let amount = if scaled >= 0.0 {
            (scaled + 0.5) as i64
        } else {
            (scaled - 0.5) as i64
        };
        Self(amount)
    }

    pub fn to_decimal(&self) -> f64 {
        self.0 as f64 / 10_f64.powi(Self::BTC_DECIMALS)
    }

    pub fn to_decimal_str(self) -> String {
        format!("{:.8}", self.to_decimal())
    }

    pub fn value(&self) -> i64 {
        self.0
    }

    pub fn is_zero(&self) -> bool {
        self.0 == 0
    }
}

impl Add<BtcAmount> for BtcAmount {
    type Output = BtcAmount;

    fn add(self, rhs: BtcAmount) -> Self::Output {
        BtcAmount::new(self.0 + rhs.0)
    }
}

impl Sub<BtcAmount> for BtcAmount {
    type Output = BtcAmount;

    fn sub(self, rhs: BtcAmount) -> Self::Output {
        BtcAmount::new(self.0 - rhs.0)
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Default, Hash)]
pub struct BtcPrice(BtcAmount);

impl BtcPrice {
    pub fn new(raw: i64) -> Self {
        Self(BtcAmount(raw))
    }

    pub fn zero() -> Self {
        Self(BtcAmount::zero())
    }

    pub fn value(&self) -> i64 {
        self.0.0
    }

    pub fn from_decimal(value: f64) -> Self {
        let scaled = value * 10_f64.powi(BtcAmount::BTC_DECIMALS);
        // Add 0.5 for proper rounding instead of truncation
        let amount = if scaled >= 0.0 {
            (scaled + 0.5) as i64
        } else {
            (scaled - 0.5) as i64
        };
        Self(BtcAmount(amount))
    }

    pub fn from_base_to_nano_btc_ratio(
        base: &InstrumentAmount,
        quote_amount_in_nano_btc: &NanoBtcAmount,
    ) -> Self {
        let shift_multiplier = 10_i64.pow((base.decimals - 1) as u32);
        if base.amount != 0 {
            let raw_price = quote_amount_in_nano_btc.0 * shift_multiplier / base.amount();
            Self::new(raw_price)
        } else {
            panic!("Cannot calculate price from zero base amount")
        }
    }

    pub fn to_decimal(self) -> f64 {
        self.0.to_decimal()
    }

    pub fn is_multiple_of(&self, price: &BtcPrice) -> bool {
        self.0.0 % price.0.0 == 0
    }

    pub fn is_zero(&self) -> bool {
        self.0.0 == 0
    }

    pub fn base_to_quote(&self, base: &InstrumentAmount) -> NanoBtcAmount {
        let shift_multiplier = 10_i64.pow((base.decimals - 1) as u32);
        NanoBtcAmount(self.0.0 * base.amount() / shift_multiplier)
    }

    pub fn quote_to_base(
        &self,
        btc_amount: BtcAmount,
        quantity_tick_size: InstrumentAmount,
    ) -> InstrumentAmount {
        if self.is_zero() {
            return InstrumentAmount::zero(quantity_tick_size.decimals);
        }

        let den = self.0.0;
        let mult = 10_i64.pow(quantity_tick_size.decimals as u32);
        if let Some(contracts) = (mult * btc_amount.value()).checked_div(den)
            && den > 0
        {
            InstrumentAmount::new(contracts, quantity_tick_size.decimals)
                .round_down(&quantity_tick_size)
        } else {
            quantity_tick_size
        }
    }

    pub fn next_multiple(&self, tick: &BtcPrice) -> Self {
        if tick.is_zero() {
            panic!("tick size must not be zero")
        }
        let raw_tick = tick.0.0;
        BtcPrice::new(raw_tick * ((self.0.0 + raw_tick - 1) / raw_tick))
    }

    pub fn prev_multiple(&self, tick: &BtcPrice) -> Self {
        if tick.is_zero() {
            panic!("tick size must not be zero")
        }
        let raw_tick = tick.0.0;
        BtcPrice::new(raw_tick * (self.0.0 / raw_tick))
    }

    pub fn round_with_tick_size(self, tick_size: i64) -> Self {
        if tick_size == 1 {
            self
        } else {
            BtcPrice::new((self.0.0 / tick_size) * tick_size)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NanoBtcAmount(i64);

impl NanoBtcAmount {
    pub fn new(value: i64) -> Self {
        Self(value)
    }

    pub fn value(&self) -> i64 {
        self.0
    }

    pub fn zero() -> Self {
        Self(0)
    }
    // making this call explicit since it affects precision
    pub fn to_sats_round_up(self) -> BtcAmount {
        // This implementation uses a mathematical trick for performing integer division with
        // ceiling (rounding up). The formula `(x + y - 1) / y`
        // performs ceiling division for positive integers.
        // Integer division with ceiling
        BtcAmount((self.0 + 9) / 10)
    }

    pub fn to_sats_round_down(self) -> BtcAmount {
        BtcAmount((self.0) / 10)
    }

    pub fn is_positive(&self) -> bool {
        self.0 > 0
    }

    pub fn pessimistic_round(self) -> BtcAmount {
        if self.is_positive() {
            self.to_sats_round_down()
        } else {
            self.to_sats_round_up()
        }
    }
}

impl Add<NanoBtcAmount> for NanoBtcAmount {
    type Output = NanoBtcAmount;

    fn add(self, rhs: NanoBtcAmount) -> Self::Output {
        NanoBtcAmount(self.0 + rhs.0)
    }
}

impl Sub<NanoBtcAmount> for NanoBtcAmount {
    type Output = NanoBtcAmount;

    fn sub(self, rhs: NanoBtcAmount) -> Self::Output {
        NanoBtcAmount(self.0 - rhs.0)
    }
}
