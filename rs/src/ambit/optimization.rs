use super::{Program, Row};
use crate::ambit::program::{Address, BitwiseAddress, Instruction};
use rustc_hash::FxHashSet;

pub fn optimize(program: &mut Program) {
    if program.instructions.is_empty() {
        return;
    }
    let mut opt = Optimization { program };
    opt.dead_code_elimination();
    opt.merge_aap();
    opt.merge_ap_aap();
}

pub struct Optimization<'p, 'a> {
    program: &'p mut Program<'a>,
}

impl Optimization<'_, '_> {
    fn dead_code_elimination(&mut self) {
        let liveness = row_liveness(self.program);
        let mut i = 0;
        let mut liveness_i = 0;
        'outer: while liveness_i < liveness.len() - 1 {
            let instruction = self.program.instructions[i];
            let liveness_after = &liveness[liveness_i + 1];

            // if no overridden row is live after this instruction, we can remove the instruction
            for row in instruction.overridden_rows(self.program.architecture) {
                if matches!(row, Row::Out(_)) || liveness_after.contains(&row) {
                    i += 1;
                    liveness_i += 1;
                    continue 'outer;
                }
            }

            liveness_i += 1;
            self.program.instructions.remove(i);
        }
    }

    fn merge_aap(&mut self) {
        let instructions = &mut self.program.instructions;
        let mut i = 0;
        while i < instructions.len() {
            let instruction = instructions[i];

            // let's just handle the simple case where we copy from a single row activation to some
            // new bitwise operand
            let Instruction::AAP(from_address, to) = instruction else {
                i += 1;
                continue;
            };
            let Some(from) = from_address.as_single_row() else {
                i += 1;
                continue;
            };
            let Address::Bitwise(BitwiseAddress::Single(to)) = to else {
                i += 1;
                continue;
            };

            let mut target_addresses = FxHashSet::default();
            target_addresses.insert(to);
            instructions.remove(i);

            // search for other operand copies from the copied-from address
            // note that we can only replace these copies if the target operand is neither read from
            // nor written to between the current instruction and the candidate instruction
            let mut candidate_i = i;
            let mut used_rows = FxHashSet::default();
            while candidate_i < instructions.len() {
                let candidate = instructions[candidate_i];
                if candidate
                    .overridden_rows(self.program.architecture)
                    .any(|row| row == from.row())
                {
                    break;
                }
                let target = || {
                    let Instruction::AAP(candidate_from, candidate_to) = candidate else {
                        return None;
                    };
                    let Address::Bitwise(BitwiseAddress::Single(candidate_to)) = candidate_to
                    else {
                        return None;
                    };
                    if from_address == candidate_from
                        && !used_rows.contains(&Row::Bitwise(candidate_to.row()))
                    {
                        Some(candidate_to)
                    } else {
                        None
                    }
                };
                let target = target();
                used_rows.extend(
                    candidate
                        .used_addresses(self.program.architecture)
                        .map(|addr| addr.row()),
                );
                match target {
                    Some(to) => {
                        target_addresses.insert(to);
                        instructions.remove(candidate_i);
                    }
                    None => {
                        candidate_i += 1;
                    }
                }
            }

            // now let's find a covering of the operators and replace the instructions accordingly
            let mut copied = FxHashSet::default();
            for (activation_i, multi_activation) in self
                .program
                .architecture
                .multi_activations
                .iter()
                .enumerate()
            {
                if multi_activation
                    .iter()
                    .any(|op| !target_addresses.contains(op))
                {
                    continue;
                }
                // TODO: not all such activation may be necessary
                copied.extend(multi_activation.iter().copied());
                instructions.insert(
                    i,
                    Instruction::AAP(
                        from.into(),
                        Address::Bitwise(BitwiseAddress::Multiple(activation_i)),
                    ),
                );
                i += 1;
            }
            for op in target_addresses.difference(&copied) {
                instructions.insert(
                    i,
                    Instruction::AAP(from.into(), Address::Bitwise(BitwiseAddress::Single(*op))),
                );
                i += 1;
            }
        }
    }

    fn merge_ap_aap(&mut self) {
        let instructions = &mut self.program.instructions;
        let mut i = 0;

        'outer: while i < instructions.len() {
            let instruction = instructions[i];
            let Instruction::AP(address) = instruction else {
                i += 1;
                continue;
            };
            let Address::Bitwise(BitwiseAddress::Multiple(mult_i)) = address else {
                i += 1;
                continue;
            };
            let mut operands = self.program.architecture.multi_activations[mult_i].clone();
            let mut used_rows = FxHashSet::default();
            for candidate_i in i + 1..instructions.len() {
                if operands.is_empty() {
                    break;
                }
                let candidate = instructions[candidate_i];

                if let Instruction::AAP(Address::Bitwise(BitwiseAddress::Single(operand)), target) =
                    candidate
                {
                    if target
                        .row_addresses(self.program.architecture)
                        .any(|addr| used_rows.contains(&addr.row()))
                        && operands.contains(&operand)
                    {
                        instructions[i] = Instruction::AAP(address, target);
                        instructions.remove(candidate_i);
                        i += 1;
                        continue 'outer;
                    }
                }

                used_rows.extend(
                    candidate
                        .used_addresses(self.program.architecture)
                        .map(|add| add.row()),
                );
                for row in candidate.overridden_rows(self.program.architecture) {
                    let mut i = 0;
                    while i < operands.len() {
                        if Row::Bitwise(operands[i].row()) == row {
                            operands.swap_remove(i);
                        } else {
                            i += 1;
                        }
                    }
                }
            }
            i += 1;
        }
    }
}

/// Returns a vector of the same length as the program where the entry at the index of an
/// instruction the set at that index contains the rows that are live just before the instruction
fn row_liveness(program: &Program) -> Vec<FxHashSet<Row>> {
    let mut result = Vec::with_capacity(program.instructions.len());

    let mut currently_live = FxHashSet::default();
    for instruction in program.instructions.iter().rev() {
        let mut is_live = false;
        for row in instruction.overridden_rows(program.architecture) {
            if matches!(row, Row::Out(_)) || currently_live.contains(&row) {
                is_live = true;
            }
            currently_live.remove(&row);
        }
        if is_live {
            currently_live.extend(
                instruction
                    .input_operands(program.architecture)
                    .map(|addr| addr.row()),
            );
        }
        result.push(currently_live.clone());
    }

    result.reverse();
    result
}
