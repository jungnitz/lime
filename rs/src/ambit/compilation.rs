use super::{
    optimization::optimize, Address, Architecture, BitwiseOperand, Program, ProgramState,
    SingleRowAddress,
};
use crate::ambit::rows::Row;
use eggmock::{Id, Mig, NetworkWithBackwardEdges, Node, Signal};
use rustc_hash::{FxHashMap, FxHashSet};
use std::cmp::max;

pub fn compile<'a>(
    architecture: &'a Architecture,
    network: &impl NetworkWithBackwardEdges<Node = Mig>,
) -> Program<'a> {
    let mut state = CompilationState::new(architecture, network);
    let mut max_cand_size = 0;
    while !state.candidates.is_empty() {
        max_cand_size = max(max_cand_size, state.candidates.len());
        let (id, node, _, _, _) = state
            .candidates
            .iter()
            .copied()
            .map(|(id, node)| {
                let outputs = state.network.node_outputs(id).count();
                let output = state.network.outputs().any(|out| out.node_id() == id);
                let not_present = node
                    .inputs()
                    .iter()
                    .map(|signal| {
                        let present = state
                            .program
                            .rows()
                            .get_rows(*signal)
                            .any(|row| matches!(row, Row::Bitwise(_)));
                        !present as u8
                    })
                    .sum::<u8>();
                (id, node, not_present, outputs, output)
            })
            .min_by_key(|(_, _, not_present, outputs, output)| (*not_present, *outputs, !output))
            .unwrap();
        let output = state.outputs.get(&id).copied();
        if let Some((output, signal)) = output {
            if signal.is_inverted() {
                state.compute(id, node, None);
                state.program.signal_copy(
                    signal,
                    SingleRowAddress::Out(output),
                    state.program.rows().get_free_dcc().unwrap_or(0),
                );
            } else {
                state.compute(id, node, Some(Address::Out(output)));
            }
            let leftover_uses = *state.leftover_use_count(id);
            if leftover_uses == 1 {
                state.program.free_id_rows(id);
            }
        } else {
            state.compute(id, node, None);
        }
    }
    let mut program = state.program.into();
    optimize(&mut program);
    program
}

pub struct CompilationState<'a, 'n, P> {
    network: &'n P,
    candidates: FxHashSet<(Id, Mig)>,
    program: ProgramState<'a>,

    outputs: FxHashMap<Id, (u64, Signal)>,
    leftover_use_count: FxHashMap<Id, usize>,
}

impl<'a, 'n, P: NetworkWithBackwardEdges<Node = Mig>> CompilationState<'a, 'n, P> {
    /// - `candidates`: , computed from `network
    /// - `outputs`: direktly read-out from `network`
    pub fn new(architecture: &'a Architecture, network: &'n P) -> Self {
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

    pub fn compute(&mut self, id: Id, node: Mig, out_address: Option<Address>) {
        if !self.candidates.remove(&(id, node)) {
            panic!("not a candidate");
        }
        let Mig::Maj(mut signals) = node else {
            panic!("can only compute majs")
        };

        // select which MAJ instruction to use
        // for this we use the operation with has the most already correctly placed operands
        let mut opt = None;
        for id in self.architecture().maj_ops.iter().copied() {
            let operands = self.architecture().multi_activations[id]
                .as_slice()
                .try_into()
                .expect("maj has to have 3 operands");
            let (matches, match_no) = self.get_mapping(&mut signals, operands);
            let dcc_cost = self.optimize_dcc_usage(&mut signals, operands, &matches);
            let spilling_cost = self.spilling_cost(operands, &matches);
            let cost = 3.0 - match_no as f32 + dcc_cost as f32 + 0.5 * spilling_cost as f32;
            let is_opt = match &opt {
                None => true,
                Some((opt_no, _, _, _)) => *opt_no > cost,
            };
            if is_opt {
                opt = Some((cost, id, matches, signals));
            }
        }
        let (_, maj_id, matches, signals) = opt.unwrap();
        let operands = &self.architecture().multi_activations[maj_id];

        // now we need to place the remaining non-matching operands...

        // for that we first find a free DCC row for possibly inverting missing signals without
        // accidentally overriding a signal that is already placed correctly
        let used_dcc = || {
            operands.iter().filter_map(|op| match op {
                BitwiseOperand::DCC { index: i, .. } => Some(i),
                _ => None,
            })
        };
        let free_dcc = (0..self.architecture().num_dcc)
            .find(|i| !used_dcc().any(|used| *used == *i))
            .expect("cannot use all DCC rows in one MAJ operation");

        // then we can copy the signals into their places
        for i in 0..3 {
            if matches[i] {
                continue;
            }
            self.program
                .signal_copy(signals[i], SingleRowAddress::Bitwise(operands[i]), free_dcc);
        }

        // all signals are in place, now we can perform the MAJ operation
        self.program
            .maj(maj_id, Signal::new(id, false), out_address);

        // free up rows if possible
        // (1) for the MAJ-signal
        if *self.leftover_use_count(id) == 0 {
            self.program.free_id_rows(id);
        }
        // (2) for the input signals
        'outer: for i in 0..3 {
            // decrease use count only once per id
            for j in 0..i {
                if signals[i].node_id() == signals[j].node_id() {
                    continue 'outer;
                }
            }
            *self.leftover_use_count(signals[i].node_id()) -= 1
        }

        // lastly, determine new candidates
        for parent_id in self.network.node_outputs(id) {
            let parent_node = self.network.node(parent_id);
            if parent_node
                .inputs()
                .iter()
                .all(|s| self.program.rows().contains_id(s.node_id()))
            {
                self.candidates.insert((parent_id, parent_node));
            }
        }
    }

    fn optimize_dcc_usage(
        &self,
        signals: &mut [Signal; 3],
        operands: &[BitwiseOperand; 3],
        matching: &[bool; 3],
    ) -> i32 {
        // first, try using a DCC row for all non-matching rows that require inversion
        let mut dcc_adjusted = [false; 3];
        let mut changed = true;
        while changed {
            changed = false;
            for i in 0..3 {
                if matching[i]
                    || operands[i].is_dcc()
                    || self.program.rows().get_rows(signals[i]).next().is_some()
                {
                    continue;
                }
                // the i-th operand needs inversion. let's try doing this by swapping it with a
                // signal of a DCC row so that we require one less copy operation
                for j in 0..3 {
                    if i == j || matching[j] || dcc_adjusted[j] {
                        continue;
                    }
                    if operands[j].is_dcc() {
                        signals.swap(i, j);
                        dcc_adjusted[j] = true;
                        changed = true;
                    }
                }
            }
        }

        let mut cost = 0;
        for ((signal, operand), matching) in signals.iter().zip(operands).zip(matching) {
            if *matching || operand.is_dcc() {
                continue;
            }
            // if the signal is not stored somewhere, i.e. only the inverted signal is present, this
            // requires a move via a DCC row to the actual operand
            if self.program.rows().get_rows(*signal).next().is_none() {
                cost += 1
            }
        }
        cost
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
