use super::{instruction::IrInstr, value::IrVariable};

#[derive(Debug, Clone)]
pub struct BasicBlock<'ir> {
    predecessors: Vec<&'ir BasicBlock<'ir>>,
    successors: Vec<&'ir BasicBlock<'ir>>,

    instructions: Vec<IrInstr<'ir>>,
    terminator: BasicBlockTerminator<'ir>,
}

impl<'ir> BasicBlock<'ir> {
    pub fn new() -> Self {
        Self {
            predecessors: Vec::new(),
            successors: Vec::new(),

            instructions: Vec::new(),
            terminator: BasicBlockTerminator::Return,
        }
    }

    pub fn push(&mut self, instr: IrInstr<'ir>) {
        self.instructions.push(instr);
    }
}

#[derive(Debug, Clone)]
pub enum BasicBlockTerminator<'ir> {
    Return,
    Branch(usize),
    BranchCond(usize, IrVariable<'ir>),
}
