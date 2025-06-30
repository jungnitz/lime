use std::fmt::Display;

use itertools::Itertools;
use lime_generic_def::{Architecture, BoolSet, Cell, CellType, Operand, Operation};

use crate::gp::{Ambit, AmbitCellType, FELIXCellType, FELIX};

pub fn set<CT: CellType + Display>(
    arch: &Architecture<CT>,
    cell: Cell<CT>,
    value: bool,
) -> Vec<Operation<CT>> {
    let mut operations = Vec::new();
    for operation in arch.operations().iter() {
        // Option 1: use output
        'opt1: {
            if let Some(output) = operation.output {
                if operation.input_results.unchanged() {
                    let (inputs, inverted) = match arch.outputs().fit_cell(cell) {
                        BoolSet::None => break 'opt1,
                        BoolSet::All => {
                            let Some((fn_value, ops)) = operation.input.try_fit_constants(output)
                            else {
                                break 'opt1;
                            };
                            (ops, fn_value != value)
                        }
                        BoolSet::Single(inverted) => {
                            let Some(ops) = operation
                                .input
                                .try_fit_constants_to_fn(output, value ^ inverted)
                            else {
                                break 'opt1;
                            };
                            (ops, inverted)
                        }
                    };
                    operations.push(Operation {
                        typ: operation.clone(),
                        inputs,
                        outputs: vec![Operand { cell, inverted }],
                    });
                }
            }
        }

        // Option 2: use overridden input value
        // for set in sets {
        //     for (i, inverted) in set.positions(cell) {
        //         let result = &operation.input_results[i];
        //         let InputResult::Function(function) = result else {
        //             continue;
        //         };
        //         // function.try_compute(value ^ inverted, operation.input, candidate_fn)
        //     }
        // }
    }
    println!("{}", operations.iter().format("\n"));
    operations
}

#[test]
fn test() {
    let ambit = FELIX::new();
    set(&ambit, Cell::new(FELIXCellType::D, 0), true);
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
//     from: A::Cell,asd
//     to: A::Cell,
//     invert: bool,asdfasdfasdf
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
