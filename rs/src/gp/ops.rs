use crate::BoolSet;

use super::{Architecture, Function, Gate, InputResult, OperandSet, Operands, OperationType};

pub fn set<A: Architecture>(arch: &A, cell: A::Cell, value: bool) {
    for operation in arch.operations() {
        let Operands::Sets(sets) = operation.input else {
            todo!();
        };
        if let Some(output) = &operation.output {
            if operation.input_results.is_empty() {
                let gate_value = match arch.outputs().fit_cell(cell) {
                    BoolSet::None => continue,
                    BoolSet::All => None,
                    BoolSet::Single(inverted) => Some(inverted ^ output.inverted),
                }
                .map(|invert| invert ^ value);
            }
        }

        // Option 2: use overridden input value
        for set in sets {
            for (i, inverted) in set.positions(cell) {
                let result = &operation.input_results[i];
                let InputResult::Function(function) = result else {
                    continue;
                };
            }
        }
    }
}

// pub fn copy<A: Architecture>(arch: &A, from: A::Cell, to: A::Cell, invert: bool) {
//     for operation in arch.operations() {
//         match &operation.input {
//             Operands::Cross(cross) => copy_using_cross(arch, from, to, invert, operation, cross),
//             Operands::Sets(sets) => copy_using_sets(arch, from, to, invert, operation, sets),
//         }
//     }
// }
//
// pub fn copy_using_cross<A: Architecture>(
//     arch: &A,
//     from: A::Cell,
//     to: A::Cell,
//     invert: bool,
//     operation: &OperationType<A>,
//     cross: &OperandCross<A>,
// ) {
//     todo!()
// }
//
// pub fn copy_using_sets<A: Architecture>(
//     arch: &A,
//     from: A::Cell,
//     to: A::Cell,
//     invert: bool,
//     operation: &OperationType<A>,
//     sets: &[OperandSet<A>],
// ) {
//     for set in sets {}
// }
//
