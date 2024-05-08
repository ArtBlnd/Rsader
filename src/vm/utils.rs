use rune::runtime::Protocol;
use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Sub, SubAssign};

use crate::utils::Decimal;

pub fn install_module_utils(context: &mut rune::Context) {
    let mut module = rune::Module::new();

    module.ty::<Decimal>().unwrap();
    module.function_meta(Decimal::normalize_).unwrap();
    module.function_meta(Decimal::abs_).unwrap();
    module.function_meta(Decimal::from_str_).unwrap();

    module
        .associated_function(Protocol::ADD, Decimal::add)
        .unwrap();
    module
        .associated_function(Protocol::ADD_ASSIGN, Decimal::add_assign)
        .unwrap();
    module
        .associated_function(Protocol::SUB, Decimal::sub)
        .unwrap();
    module
        .associated_function(Protocol::SUB_ASSIGN, Decimal::sub_assign)
        .unwrap();
    module
        .associated_function(Protocol::MUL, Decimal::mul)
        .unwrap();
    module
        .associated_function(Protocol::MUL_ASSIGN, Decimal::mul_assign)
        .unwrap();
    module
        .associated_function(Protocol::DIV, Decimal::div)
        .unwrap();
    module
        .associated_function(Protocol::DIV_ASSIGN, Decimal::div_assign)
        .unwrap();

    context.install(module).unwrap();
}

impl Decimal {
    #[rune::function(instance, path = Self::normalize)]
    fn normalize_(self) -> Decimal {
        self.normalize()
    }

    #[rune::function(instance, path = Self::abs)]
    fn abs_(self) -> Decimal {
        self.abs()
    }

    #[rune::function(instance, path = Self::from_str)]
    fn from_str_(s: &str) -> Decimal {
        Decimal::from_str(s).unwrap()
    }
}
