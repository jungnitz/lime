use std::fmt::{self, Display, Formatter};
use std::hash::Hash;
use std::{fmt::Debug, ops::Deref};

use itertools::Format;

use super::Architecture;

pub type CellIndex = u32;

pub trait CellType: Copy + Debug + PartialEq + Eq + Hash {
    type Arch: Architecture<CellType = Self>;
    const CONSTANT: Self;

    /// Number of cells of this type or `None` if infinite amount is available.
    fn count(self) -> Option<CellIndex>;
    fn name(self) -> &'static str;
}

pub trait Cell: Copy + Debug + PartialEq + Eq + Hash {
    type Arch: Architecture<Cell = Self>;

    fn new(typ: <Self::Arch as Architecture>::CellType, idx: CellIndex) -> Self;
    fn typ(self) -> <Self::Arch as Architecture>::CellType;
    fn index(self) -> CellIndex;
    /// Returns the value of this cell if it is a constant or `None` if it is not.
    fn constant_value(self) -> Option<bool> {
        if self.typ() == <Self::Arch as Architecture>::CellType::CONSTANT {
            Some(self.index() != 0)
        } else {
            None
        }
    }
}

#[doc(hidden)]
pub fn __display_cell_type<T: CellType>(typ: T, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
    write!(f, "{}", typ.name())
}

#[doc(hidden)]
pub fn __display_cell<C: Cell>(cell: &C, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
    __display_cell_type(cell.typ(), f)?;
    write!(f, "[{}]", cell.index())
}
