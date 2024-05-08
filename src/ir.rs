use self::{arena::IrArena, function::IrFunction, scope::Scope, ty::IrType, value::IrVariable};

pub mod arena;
pub mod basic_block;
pub mod function;
pub mod instruction;
pub mod lifter;
pub mod scope;
pub mod ty;
pub mod value;

pub struct Ir<'ir> {
    functions: Vec<&'ir mut IrFunction<'ir>>,
}

impl<'ir> Ir<'ir> {}
