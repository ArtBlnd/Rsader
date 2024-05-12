use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Sub, SubAssign};

use crate::utils::Decimal;

use rune::alloc::fmt::TryWrite;
use rune::runtime::{Formatter, Protocol, VmResult};

use super::error;

pub fn install_module_utils(context: &mut rune::Context) {
    let mut module = rune::Module::new();

    module.ty::<Decimal>().unwrap();
    module.function_meta(Decimal::normalize__meta).unwrap();
    module.function_meta(Decimal::abs__meta).unwrap();
    module.function_meta(Decimal::round_dp__meta).unwrap();
    module.function_meta(Decimal::decimal_from_str).unwrap();
    module.function_meta(Decimal::string_display).unwrap();

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
    #[rune::function(path = Decimal::from_str)]
    fn decimal_from_str(s: &str) -> error::Result<Decimal> {
        Decimal::from_str(s).map_err(|e| error::Error::from_stderr(e))
    }

    #[rune::function(instance, protocol = STRING_DISPLAY)]
    fn string_display(&self, f: &mut Formatter) -> VmResult<()> {
        rune::vm_write!(f, "{}", self);
        VmResult::Ok(())
    }
}
