use super::{basic_block::BasicBlock, ty::IrType, value::IrVariable};

pub struct IrFunction<'ir> {
    name: String,

    param0: Vec<&'ir IrType>,
    param1: Vec<IrVariable<'ir>>,
    basic_blocks: Vec<BasicBlock<'ir>>,
}

impl<'ir> IrFunction<'ir> {
    pub fn new(
        param0: Vec<&'ir IrType>,
        param1: Vec<IrVariable<'ir>>,
        name: impl AsRef<str>,
    ) -> Self {
        Self {
            name: name.as_ref().to_string(),

            param0,
            param1,
            basic_blocks: Vec::new(),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn param1(&self) -> &[IrVariable<'ir>] {
        &self.param1
    }
}
