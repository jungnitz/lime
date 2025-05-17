//!
//! - [`compile()`] = main function - compiles given logic network for the given [`architecture`]
//! into a [`program`] using some [`optimization`]

use super::{
    architecture::{FCDRAMArchitecture, Instruction}, optimization::optimize, Program, ProgramState, RowAddress
};
use eggmock::{Id, Aig, Node, NetworkWithBackwardEdges, Signal};
use rustc_hash::{FxHashMap, FxHashSet};
use std::cmp::max;

/// Compiles given `network` intto a FCDRAM-[`Program`] that can be run on given `architecture`
pub fn compile(
    network: &impl NetworkWithBackwardEdges<Node = Aig>,
) -> Program {

    let (outputs, leaves) = (network.outputs(), network.leaves());
    Program {
        instructions: vec!(Instruction::FracOp(-1)) ,
    }
    // todo!()
    // optimize(&mut program);
    // program
}

pub struct CompilationState<'n, P> {
    /// Network (P=Provider, obsolte naming)
    network: &'n P,
    candidates: FxHashSet<(Id, Aig)>,
    program: ProgramState,

    outputs: FxHashMap<Id, (u64, Signal)>,
    leftover_use_count: FxHashMap<Id, usize>,
}

impl<'n, P: NetworkWithBackwardEdges<Node = Aig>> CompilationState<'n, P> {
    pub fn new(network: &'n P) -> Self {
        let mut candidates = FxHashSet::default();
        // check all parents of leaves whether they have only leaf children, in which case they are
        // candidates
        for leaf in network.leaves() {
            for candidate_id in network.node_outputs(leaf) {
                let candidate = network.node(candidate_id);
                if candidate
                    .inputs()
                    .iter()
                    .all(|signal| network.node(signal.node_id()).is_leaf())
                {
                    candidates.insert((candidate_id, candidate));
                }
            }
        }
        let program = ProgramState::new(network);

        let outputs = network
            .outputs()
            .enumerate()
            .map(|(id, sig)| (sig.node_id(), (id as u64, sig)))
            .collect();

        Self {
            network,
            candidates,
            program,
            outputs,
            leftover_use_count: FxHashMap::default(),
        }
    }
}
