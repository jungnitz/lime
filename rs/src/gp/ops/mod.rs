mod constant;
mod copy;

use std::fmt::Display;

use itertools::Itertools;
use lime_generic_def::{Cell, Operand};

use crate::gp::{
    Ambit, AmbitCellType, FELIX, FELIXCellType, IMPLY, IMPLYCellType, PLiM, PLiMCellType,
    ops::{constant::set, copy::copy},
    program::{Program, ProgramVersion, ProgramVersions},
};

#[test]
fn test() {
    let mut program = Program::new(IMPLY::new());
    copy(
        &mut program.append_with_min_cost(),
        Cell::new(IMPLYCellType::D, 1),
        Operand {
            cell: Cell::new(IMPLYCellType::D, 2),
            inverted: true,
        },
    );
    println!("{program}")
}
