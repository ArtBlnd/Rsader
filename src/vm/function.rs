use crate::ir::ty::IrType;

use super::{
    action::TaskContext,
    builtin_functions::{self, BuiltinFunction},
    context::VmContext,
    value::Value,
    VmError,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Function {
    Builtin(BuiltinFunction),
}

impl Function {
    pub fn name(&self) -> String {
        match self {
            Function::Builtin(builtin) => builtin.to_string(),
        }
    }

    pub fn ty(&self) -> IrType {
        match self {
            Function::Builtin(builtin) => builtin.ty(),
        }
    }

    pub fn display_name(&self) -> String {
        format!("{}{}", self.name(), self.ty())
    }
}

impl Function {
    pub async fn call<'ctx>(
        &self,
        task_context: &TaskContext,
        args0: Vec<IrType>,
        args1: Vec<Value>,
        ctx: &'ctx mut VmContext,
    ) -> Result<Value, VmError> {
        match self {
            Function::Builtin(builtin) => {
                builtin_functions::execute_builtins(
                    builtin.clone(),
                    ctx,
                    task_context,
                    args0,
                    args1,
                )
                .await?
            }
        }
        .ok_or_else(|| VmError::Panic(self.display_name()))
    }
}
