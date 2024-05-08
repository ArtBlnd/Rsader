use crate::{
    ast::{BinaryOp, ParseError},
    ir::ty::IrType,
};

pub mod action;
pub mod builtin_functions;
pub mod builtin_ident;
pub mod builtin_types;
pub mod context;
pub mod eval_ast;
pub mod function;
pub mod value;

#[derive(thiserror::Error, Debug, PartialEq, Eq, Clone)]
pub enum VmError {
    #[error("{0}")]
    ParseError(#[from] ParseError),

    #[error("expected type `{0}`, but found `{1}`")]
    TypeMismatch(IrType, IrType),

    #[error("expected function, found `{0}`")]
    ExpectedFunction(IrType),

    #[error("expected function call")]
    ExpectedFunctionCall,

    #[error("invalid left-hand side of assignment")]
    InvalidAssignment,

    #[error("cannot find value `{0}` in this scope")]
    UndefinedVariable(String),

    #[error("cannot {0} on type `{1}` to type `{2}`")]
    InvalidOperation(BinaryOp, IrType, IrType),

    #[error("expected futture, found `{0}`")]
    ExpectedFuture(IrType),

    #[error("panic occurred: {0}")]
    Panic(String),
}
