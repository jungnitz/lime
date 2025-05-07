//!
//! - [`compile()`] = main function - compiles given logic network for the given [`architecture`]
//! into a [`program`] using some [`optimization`]

use super::{
    architecture::FCDRAMArchitecture, optimization::optimize, Program, ProgramState, RowAddress
};
use eggmock::{Id, Mig, Aig, Node, ProviderWithBackwardEdges, Signal};
use rustc_hash::{FxHashMap, FxHashSet};
use std::cmp::max;

/// Compiles given `network` to a program that can be run on given `architecture`
pub fn compile(
    network: &impl ProviderWithBackwardEdges<Node = Mig>,
) -> Program {
    todo!()
    // let mut state = CompilationState::new(architecture, network);
    // let mut max_cand_size = 0; // TODO: unused?
    // while !state.candidates.is_empty() {
    //     max_cand_size = max(max_cand_size, state.candidates.len());
    //
    //     // TODO: ???
    //     let (id, node, _, _, _) = state
    //         .candidates
    //         .iter()
    //         .copied()
    //         .map(|(id, node)| {
    //             let outputs = state.network.node_outputs(id).count();
    //             let output = state.network.outputs().any(|out| out.node_id() == id);
    //             let not_present = node
    //                 .inputs()
    //                 .iter()
    //                 .map(|signal| {
    //                     let present = state
    //                         .program
    //                         .rows();
    //                         // .any(|row| matches!(row, Row::Bitwise(_)));
    //                     todo!()
    //                 })
    //                 .sum::<u8>();
    //             (id, node, not_present, outputs, output)
    //         })
    //         .min_by_key(|(_, _, not_present, outputs, output)| (*not_present, *outputs, !output))
    //         .unwrap();
    //
    //     // TODO: ???
    //     let output = state.outputs.get(&id).copied();
    //     if let Some((output, signal)) = output {
    //         if signal.is_inverted() {
    //             state.compute(id, node, None);
    //         } else {
    //             // state.compute(id, node, Some(Address::Out(output)));
    //             state.compute(id, node, None);
    //         }
    //         let leftover_uses = *state.leftover_use_count(id);
    //         if leftover_uses == 1 {
    //             state.program.free_id_rows(id);
    //         }
    //     } else {
    //         state.compute(id, node, None);
    //     }
    // }
    //
    // let mut program = state.program.into();
    // optimize(&mut program);
    // program
}

pub struct CompilationState<'n, P> {
    /// Network (P=Provider, obsolte naming)
    network: &'n P,
    candidates: FxHashSet<(Id, Mig)>, // TODO: probably change to `Aig` ?
    program: ProgramState,

    outputs: FxHashMap<Id, (u64, Signal)>,
    leftover_use_count: FxHashMap<Id, usize>,
}

impl<'n, P: ProviderWithBackwardEdges<Node = Mig>> CompilationState<'n, P> {
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

    pub fn leftover_use_count(&mut self, id: Id) -> &mut usize {
        self.leftover_use_count.entry(id).or_insert_with(|| {
            self.network.node_outputs(id).count() + self.outputs.contains_key(&id) as usize
        })
    }

    pub fn compute(&mut self, id: Id, node: Mig, out_address: Option<RowAddress>) {
        todo!()
    }

    /// Reorders the `signals` so that the maximum number of the given signal-operator-pairs already
    /// match according to the current program state.
    /// The returned array contains true for each operand that then already contains the correct
    /// signal and the number is equal to the number of trues in the array.
    fn get_mapping(
        &self,
        signals: &mut [Signal; 3],
        operands: &[RowAddress; 3],
    ) -> ([bool; 3], usize) {
        todo!()
        // let signals_with_idx = {
        //     let mut i = 0;
        //     signals.map(|signal| {
        //         i += 1;
        //         (signal, i - 1)
        //     })
        // };
        // let operand_signals = operands.map(|op| self.program.rows().get_operand_signal(op));
        //
        // // reorder signals by how often their signal is already available in an operand
        // let mut signals_with_matches = signals_with_idx.map(|(s, i)| {
        //     (
        //         s,
        //         i,
        //         operand_signals
        //             .iter()
        //             .filter(|sig| **sig == Some(s))
        //             .count(),
        //     )
        // });
        // signals_with_matches.sort_by(|a, b| a.2.cmp(&b.2));
        //
        // // then we can assign places one by one and get an optimal mapping (probably, proof by
        // // intuition only)
        //
        // // contains for each operand index whether the signal at that position is already the
        // // correct one
        // let mut result = [false; 3];
        // // contains the mapping of old signal index to operand index
        // let mut new_positions = [0usize, 1, 2];
        // // contains the number of assigned signals (i.e. #true in result)
        // let mut assigned_signals = 0;
        //
        // for (signal, signal_idx, _) in signals_with_matches {
        //     // find operand index for that signal
        //     let Some((target_idx, _)) = operand_signals
        //         .iter()
        //         .enumerate()
        //         .find(|(idx, sig)| **sig == Some(signal) && !result[*idx])
        //     else {
        //         continue;
        //     };
        //     result[target_idx] = true;
        //     let new_idx = new_positions[signal_idx];
        //     signals.swap(target_idx, new_idx);
        //     new_positions.swap(target_idx, new_idx);
        //     assigned_signals += 1;
        // }
        // (result, assigned_signals)
    }
}
