#![allow(dead_code)]

mod parse_rule;
pub use parse_rule::*;
mod token;
pub use token::*;

use std::{fmt::Display, ops::Range};

use rust_decimal::Decimal;

use crate::currency::Currency;

#[derive(thiserror::Error, Debug, PartialEq, Eq, Clone)]
pub enum ParseError {
    #[error("invalid token")]
    InvalidToken(Range<usize>),
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Stmt<'input> {
    Let(Let<'input>),
    Item(Item<'input>),
    Expr(Expr<'input>),
    Empty,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Expr<'input> {
    Block(Vec<Stmt<'input>>),
    Async(Box<Expr<'input>>),
    Await(Box<Expr<'input>>),
    BinOp {
        op: BinaryOp,
        lhs: Box<Expr<'input>>,
        rhs: Box<Expr<'input>>,
    },
    UnaryOp {
        op: UnaryOp,
        expr: Box<Expr<'input>>,
    },
    Identifier {
        name: Ident<'input>,
    },
    Call {
        expr: Box<Expr<'input>>,
        args0: Vec<Ty<'input>>,
        args1: Vec<Expr<'input>>,
    },
    FieldAccess {
        expr: Box<Expr<'input>>,
        field: Ident<'input>,
    },
    MethodCall {
        expr: Box<Expr<'input>>,
        field: Ident<'input>,
        args0: Vec<Ty<'input>>,
        args1: Vec<Expr<'input>>,
    },
    Tuple {
        items: Vec<Expr<'input>>,
    },
    Literal(Literal),
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Item<'input> {
    Function {
        name: Ident<'input>,
        args0: Vec<Ty<'input>>,
        args1: Vec<NameAndTy<'input>>,
        body: Vec<Stmt<'input>>,
    },
    Struct {
        name: Ident<'input>,
        fields: Vec<NameAndTy<'input>>,
    },
    Enum {
        name: Ident<'input>,
        variants: Vec<Variant<'input>>,
    },
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Let<'input> {
    pub name: Ident<'input>,
    pub ty: Option<Ty<'input>>,
    pub expr: Option<Expr<'input>>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Ty<'input>(pub &'input str);

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct NameAndTy<'input> {
    pub name: Ident<'input>,
    pub ty: Ty<'input>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Variant<'input> {
    Unit(Ident<'input>),
    Struct(Ident<'input>, Vec<NameAndTy<'input>>),
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum BinaryOp {
    Assign,
    Arithmetic(BinaryArithmeticOp),
    Bitwise(BinaryBitwiseOp),
    Logical(BinaryLogicalOp),
    Comparison(BinaryComparisonOp),
}

impl Display for BinaryOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BinaryOp::Assign => write!(f, "="),
            BinaryOp::Arithmetic(op) => write!(f, "{:?}", op),
            BinaryOp::Bitwise(op) => write!(f, "{:?}", op),
            BinaryOp::Logical(op) => write!(f, "{:?}", op),
            BinaryOp::Comparison(op) => write!(f, "{:?}", op),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, strum::Display)]
#[strum(serialize_all = "snake_case")]
pub enum BinaryArithmeticOp {
    Add,
    Sub,
    Mul,
    Div,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, strum::Display)]
#[strum(serialize_all = "snake_case")]
pub enum BinaryBitwiseOp {
    And,
    Or,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, strum::Display)]
#[strum(serialize_all = "snake_case")]
pub enum BinaryLogicalOp {
    And,
    Or,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, strum::Display)]
#[strum(serialize_all = "snake_case")]
pub enum BinaryComparisonOp {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, strum::Display)]
#[strum(serialize_all = "snake_case")]
pub enum UnaryOp {
    Neg,
    Not,
    Ref,
    Mut,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Ident<'input>(pub &'input str);

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Literal {
    String(String),
    Boolean(bool),
    Decimal(Decimal),
    Integer(u128),
    Currency(Currency),
    CurrencyPair((Currency, Currency)),
}
