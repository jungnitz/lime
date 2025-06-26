mod cell;
mod func;
mod operand;
mod operands;
mod operation;

use std::fmt::{Display, Formatter};
use std::hash::Hash;
use std::{fmt::Debug, ops::Deref};

use derive_more::{Deref, From};

pub use self::{cell::*, func::*, operand::*, operands::*, operation::*};
use crate::BoolSet;

/// Abstractly describes a Logic-in-Memory architecture.
pub trait Architecture: Sized + 'static {
    type CellType: CellType<Arch = Self>;
    type Cell: Cell<Arch = Self>;

    fn outputs(&self) -> Operands<Self>;
    fn operations(&self) -> &'static [OperationType<Self>];

    fn cell(typ: Self::CellType, idx: CellIndex) -> Self::Cell {
        Self::Cell::new(typ, idx)
    }
    fn constant(value: bool) -> Self::Cell {
        Self::cell(Self::CellType::CONSTANT, value as u32)
    }
}

fn display_maybe_inverted<D: Display>(
    f: &mut Formatter<'_>,
    d: D,
    inverted: bool,
) -> Result<(), std::fmt::Error> {
    if inverted {
        write!(f, "!{}", d)
    } else {
        write!(f, "{}", d)
    }
}
