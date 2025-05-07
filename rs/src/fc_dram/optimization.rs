use crate::fc_dram::architecture::{RowAddress, Instruction};
use rustc_hash::FxHashSet;

use super::{architecture::FCDRAMArchitecture, program::Program};

pub fn optimize(program: &mut Program) {
    if program.instructions.len() == 0 {
        return;
    }
    let mut opt = Optimization { program };
    // TODO: perform optimizations !
}

pub struct Optimization<'p, 'a> {
    program: &'p mut Program<'a>,
}

// TODO: manual optimizations?
impl Optimization<'_, '_,> {
    /// TODO: perform some basic compiler-optimization like dead_code_elimination? or will this
    /// already be done by the MLIR dialect?
    fn dead_code_elimination(&mut self) {
        todo!()
    }
}

