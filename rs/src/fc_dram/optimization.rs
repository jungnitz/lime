//!  Optimize code from `compiler.rs`
//! - things to optimize for:
//!     - performance (nr of required mem-cycles): mostly reduce nr of RowClone-ops (rest is mostly predetermined by logic-graph due to 1:1 mapping of LogicalOps -> FCDRAM Primitives)
//!     - success rate (mostly by choosing right rows to optimize for distance to sense-amps), includes input replication
//!     - memory-footprint (reduce nr of subarrays used by program)
//!         - manually adapt safe-space to program requirements: unused safe-space rows could still be used ?!
//!         - [ ] Rematerialization ?
use crate::fc_dram::architecture::{RowAddress, Instruction};

use super::{architecture::FCDRAMArchitecture, program::Program};

pub fn optimize(program: &mut Program) {
    if program.instructions.is_empty() {
        return;
    }
    let mut opt = Optimization { program };
    // TODO: perform optimizations !
}

pub struct Optimization<'p> {
    program: &'p mut Program,
}

// TODO: manual optimizations?
impl Optimization<'_> {
    /// TODO: perform some basic compiler-optimization like dead_code_elimination? or will this
    /// already be done by the MLIR dialect?
    fn dead_code_elimination(&mut self) {
        todo!()
    }
}

