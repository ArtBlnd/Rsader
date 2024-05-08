use super::value::{IrValue, IrVariable};

#[derive(Debug, Clone)]
pub enum IrInstr<'ir> {
    BinaryOp {
        op: IrBinaryOp,
        dst: IrVariable<'ir>,
        lhs: IrValue<'ir>,
        rhs: IrValue<'ir>,
    },
    Assign {
        dst: IrVariable<'ir>,
        src: IrValue<'ir>,
    },
}

#[derive(Debug, Clone)]
pub enum IrBinaryOp {
    Assign,
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    And,
    Or,
    Xor,
    Shl,
    Shr,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}
