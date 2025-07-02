mod constant;

use std::fmt::Display;

use itertools::Itertools;
use lime_generic_def::Cell;

use crate::gp::{
    Ambit, AmbitCellType, FELIX, FELIXCellType, IMPLY, IMPLYCellType, ops::constant::set,
};

#[test]
fn test() {
    let ambit = IMPLY::new();
    set(&ambit, Cell::new(IMPLYCellType::D, 1), false);
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
