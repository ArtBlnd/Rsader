use rust_decimal::Decimal;

use crate::currency::Currency;

use super::ty::IrType;

#[derive(Debug, Clone, Copy)]
pub enum IrValue<'ir> {
    Constant(IrConstant<'ir>),
    Variable(IrVariable<'ir>),
}

#[derive(Debug, Clone, Copy)]
pub enum IrConstant<'ir> {
    Decimal(Decimal),
    Integer(u128, &'ir IrType),
    String(&'ir str),
    Boolean(bool),
    Currency(Currency),
}

#[derive(Debug, Clone, Copy)]
pub struct IrVariable<'ir> {
    index: usize,
    ty: &'ir IrType,
}

impl<'ir> IrVariable<'ir> {
    pub fn new(index: usize, ty: &'ir IrType) -> Self {
        Self { index, ty }
    }

    pub fn ty(&self) -> &'ir IrType {
        self.ty
    }
}
