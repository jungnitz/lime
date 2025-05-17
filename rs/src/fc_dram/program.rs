//! Functionality for generating actual program using architecture defined in [`architecture`] by
//! compiling given logic-network (see [`compilation`]) and potentially adding some manual
//! optimizations ([`optimization`])
use super::architecture::{FCDRAMArchitecture, RowAddress};
use crate::fc_dram::architecture::Instruction;
use eggmock::{Id, Aig, NetworkWithBackwardEdges, Signal};
use std::fmt::{Display, Formatter};
use std::ops::{Deref, DerefMut};


#[derive(Debug, Clone)]
pub struct Program {
    pub instructions: Vec<Instruction>,
}

#[derive(Debug, Clone)]
pub struct ProgramState {
    program: Program,
    /// currently used rows
    rows: Vec<RowAddress>,
}

impl Program {
    pub fn new(instructions: Vec<Instruction>) -> Self {
        Self {
            instructions,
        }
    }
}

impl ProgramState {
    pub fn new(
        network: &impl NetworkWithBackwardEdges<Node = Aig>,
    ) -> Self {
        Self {
            program: Program::new(Vec::new()),
            rows: vec!(),
        }
    }


    /// TODO: Does FC-DRAM need copying of signals?
    /// pub fn signal_copy(&mut self, signal: Signal, target: RowAddress, intermediate_dcc: u8) {
    /// }

    /// Sets the value of the operand in `self.rows` to the given signal. If that removes the last
    /// reference to the node of the previous signal of the operator, insert spill code for the
    /// previous signal
    /// **ALWAYS** call this before inserting the actual instruction, otherwise the spill code will
    /// spill the wrong value
    fn set_signal(&mut self, address: RowAddress, signal: Signal) {
        todo!()
    }

    /// return rows which are currently unused (so they can be used for operations to come)
    pub fn free_id_rows(&mut self, id: Id) {
        todo!()
    }

    pub fn rows(&self) -> &Vec<RowAddress> {
        &self.rows
    }
}

impl Deref for ProgramState {
    type Target = Program;

    fn deref(&self) -> &Self::Target {
        &self.program
    }
}

impl DerefMut for ProgramState {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.program
    }
}

impl From<ProgramState> for Program {
    fn from(value: ProgramState) -> Self {
        value.program
    }
}

/// Print the generated program in human-readable form
impl Display for Program {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for instr in &self.instructions {
            match instr {
                Instruction::FracOp(row) => write!(f, "AP({row})")?, // TODO: repeat this by the factor set in compiler-settings
                Instruction::APA(row1, row2, ) => write!(f, "APA({row1},{row2})")?,
            }
        }
        Ok(())
    }
}
