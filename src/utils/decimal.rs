use std::{
    fmt::{self, Display, Formatter},
    iter::Sum,
    ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Sub, SubAssign},
    str::FromStr,
};

use num_traits::{One, ToPrimitive, Zero};
use serde::{Deserialize, Serialize};

#[derive(
    Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, rune::Any,
)]
pub struct Decimal(pub rust_decimal::Decimal);

impl Decimal {
    pub const ZERO: Self = Decimal(rust_decimal::Decimal::ZERO);
    pub const ONE: Self = Decimal(rust_decimal::Decimal::ONE);

    pub fn normalize(self) -> Self {
        Decimal(self.0.normalize())
    }

    pub fn round_dp(self, dp: u32) -> Self {
        Decimal(self.0.round_dp(dp))
    }

    pub fn from_str(s: &str) -> Result<Self, rust_decimal::Error> {
        Ok(Decimal(rust_decimal::Decimal::from_str(s)?))
    }

    pub fn abs(self) -> Self {
        Decimal(self.0.abs())
    }
}

impl One for Decimal {
    fn one() -> Self {
        Decimal(rust_decimal::Decimal::ONE)
    }
}

impl Zero for Decimal {
    fn zero() -> Self {
        Decimal(rust_decimal::Decimal::ZERO)
    }

    fn is_zero(&self) -> bool {
        self.0.is_zero()
    }
}

impl ToPrimitive for Decimal {
    fn to_i64(&self) -> Option<i64> {
        self.0.to_i64()
    }

    fn to_u64(&self) -> Option<u64> {
        self.0.to_u64()
    }
}

impl Add for Decimal {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        Decimal(self.0 + rhs.0)
    }
}

impl AddAssign for Decimal {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0;
    }
}

impl Sub for Decimal {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self {
        Decimal(self.0 - rhs.0)
    }
}

impl SubAssign for Decimal {
    fn sub_assign(&mut self, rhs: Self) {
        self.0 -= rhs.0;
    }
}

impl Mul for Decimal {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self {
        Decimal(self.0 * rhs.0)
    }
}

impl MulAssign for Decimal {
    fn mul_assign(&mut self, rhs: Self) {
        self.0 *= rhs.0;
    }
}

impl Div for Decimal {
    type Output = Self;

    fn div(self, rhs: Self) -> Self {
        Decimal(self.0 / rhs.0)
    }
}

impl DivAssign for Decimal {
    fn div_assign(&mut self, rhs: Self) {
        self.0 /= rhs.0;
    }
}

impl Sum for Decimal {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        Decimal(iter.map(|x| x.0).sum())
    }
}

impl Display for Decimal {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[macro_export]
macro_rules! dec {
    ($x:expr) => {
        crate::utils::Decimal(rust_decimal_macros::dec!($x))
    };
}
