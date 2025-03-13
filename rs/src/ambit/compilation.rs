use super::{
    Address, Architecture, BitwiseOperand, Program, ProgramState,
    SingleRowAddress,
};
use crate::ambit::rows::Row;
use eggmock::{Id, MigNode, Node, ProviderWithBackwardEdges, Signal};
use rustc_hash::{FxHashMap, FxHashSet};
use crate::ambit::optimization::optimize;

pub fn compile<'a>(
    architecture: &'a Architecture,
    network: &impl ProviderWithBackwardEdges<Node = MigNode>,
) -> Program<'a> {
    let mut state = CompilationState::new(architecture, network);
    while !state.candidates.is_empty() {
        // select candidate and operation to calculate that candidate
        let (_, &id, &node, op_idx) = state.candidates.iter().map(|(id, node)| {
            let (len, op_idx) = architecture.maj_ops.iter().copied().map(|op_idx| {
                state.program.snapshot();
                state.program.compute(Signal::new(*id, false), *node, None, op_idx);
                let len = state.program.instructions.len();
                state.program.rollback();
                (len, op_idx)
            }).min_by_key(|v| v.0).unwrap();
            (len, id, node, op_idx)
        }).min_by_key(|v| v.0).unwrap();

        let MigNode::Maj(signals) = node else {
            panic!("not a maj");
        };
        let node_signal = Signal::new(id, false);

        // perform actual calculation
        let output = state.outputs.get(&id).copied();
        if let Some((output, signal)) = output {
            if signal.is_inverted() {
                state.program.compute(node_signal, node, None, op_idx);
                state.program.signal_copy(
                    signal,
                    SingleRowAddress::Out(output),
                    state.program.rows().get_free_dcc().unwrap_or(0),
                );
            } else {
                state.program.compute(node_signal, node, Some(Address::Out(output)), op_idx);
            }
            let leftover_uses = *state.leftover_use_count(id);
            if leftover_uses == 1 {
                state.program.free_id_rows(id);
            }
        } else {
            state.program.compute(node_signal, node, None, op_idx);
        }


        // update candidates
        state.candidates.remove(&(id, node));

        // free up rows if possible
        // (1) for the MAJ-signal
        if *state.leftover_use_count(id) == 0 {
            state.program.free_id_rows(id);
        }
        // (2) for the input signals
        'outer: for i in 0..3 {
            // decrease use count only once per id
            for j in 0..i {
                if signals[i].node_id() == signals[j].node_id() {
                    continue 'outer;
                }
            }
            *state.leftover_use_count(signals[i].node_id()) -= 1
        }

        // lastly, determine new candidates
        for parent_id in state.network.node_outputs(id) {
            let parent_node = state.network.node(parent_id);
            if parent_node
                .inputs()
                .iter()
                .all(|s| state.program.rows().contains_id(s.node_id()))
            {
                state.candidates.insert((parent_id, parent_node));
            }
        }
    }
    let mut program = state.program.into();
    optimize(&mut program);
    program
}

pub struct CompilationState<'a, 'n, P> {
    network: &'n P,
    candidates: FxHashSet<(Id, MigNode)>,
    program: ProgramState<'a>,

    outputs: FxHashMap<Id, (u64, Signal)>,
    leftover_use_count: FxHashMap<Id, usize>,
}

impl<'a, 'n, P: ProviderWithBackwardEdges<Node = MigNode>> CompilationState<'a, 'n, P> {
    pub fn new(architecture: &'a Architecture, network: &'n P) -> Self {
        let mut candidates = FxHashSet::default();
        // check all parents of leafs whether they have only leaf children, in which case they are
        // candidates
        for leaf in network.leafs() {
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
        let program = ProgramState::new(architecture, network);

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

    fn spilling_cost(&self, operands: &[BitwiseOperand; 3], matching: &[bool; 3]) -> i32 {
        let mut cost = 0;
        for i in 0..3 {
            if matching[i] {
                continue;
            }
            let Some(_signal) = self
                .program
                .rows()
                .get_row_signal(Row::Bitwise(operands[i].row()))
            else {
                continue;
            };
            cost += 1
            // // signal and inverted signal not present somewhere else
            // if self.program.rows().get_rows(signal).count() < 2
            //     && self
            //         .program
            //         .rows()
            //         .get_rows(signal.invert())
            //         .next()
            //         .is_none()
            // {
            //     cost += 1
            // }
        }
        cost
    }

    /// Reorders the `signals` so that the maximum number of the given signal-operator-pairs already
    /// match according to the current program state.
    /// The returned array contains true for each operand that then already contains the correct
    /// signal and the number is equal to the number of trues in the array.
    fn get_mapping(
        &self,
        signals: &mut [Signal; 3],
        operands: &[BitwiseOperand; 3],
    ) -> ([bool; 3], usize) {
        let signals_with_idx = {
            let mut i = 0;
            signals.map(|signal| {
                i += 1;
                (signal, i - 1)
            })
        };
        let operand_signals = operands.map(|op| self.program.rows().get_operand_signal(op));

        // reorder signals by how often their signal is already available in an operand
        let mut signals_with_matches = signals_with_idx.map(|(s, i)| {
            (
                s,
                i,
                operand_signals
                    .iter()
                    .filter(|sig| **sig == Some(s))
                    .count(),
            )
        });
        signals_with_matches.sort_by(|a, b| a.2.cmp(&b.2));

        // then we can assign places one by one and get an optimal mapping (probably, proof by
        // intuition only)

        // contains for each operand index whether the signal at that position is already the
        // correct one
        let mut result = [false; 3];
        // contains the mapping of old signal index to operand index
        let mut new_positions = [0usize, 1, 2];
        // contains the number of assigned signals (i.e. #true in result)
        let mut assigned_signals = 0;

        for (signal, signal_idx, _) in signals_with_matches {
            // find operand index for that signal
            let Some((target_idx, _)) = operand_signals
                .iter()
                .enumerate()
                .find(|(idx, sig)| **sig == Some(signal) && !result[*idx])
            else {
                continue;
            };
            result[target_idx] = true;
            let new_idx = new_positions[signal_idx];
            signals.swap(target_idx, new_idx);
            new_positions.swap(target_idx, new_idx);
            assigned_signals += 1;
        }
        (result, assigned_signals)
    }

    fn architecture(&self) -> &'a Architecture {
        self.program.architecture
    }
}
