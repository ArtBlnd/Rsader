use std::pin::Pin;

use self_reference::RefDef;
use typed_arena::Arena;

use super::{function::IrFunction, ty::IrType};

pub struct IrArena<'ir> {
    string_literal: Arena<String>,
    functions: Arena<IrFunction<'ir>>,
    types: Arena<IrType>,
}

impl<'ir> IrArena<'ir> {
    pub fn new() -> Self {
        Self {
            string_literal: Arena::new(),

            functions: Arena::new(),
            types: Arena::new(),
        }
    }

    pub fn alloc_string(&'ir self, string: String) -> &'ir str {
        self.string_literal.alloc(string)
    }

    pub fn alloc_function(&'ir self, function: IrFunction<'ir>) -> &'ir mut IrFunction<'ir> {
        self.functions.alloc(function)
    }

    pub fn alloc_type(&'ir self, types: IrType) -> &'ir IrType {
        self.types.alloc(types)
    }
}
