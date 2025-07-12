use std::borrow::Cow;
use std::fmt::{self, Debug, Display, Formatter};
use std::hash::Hash;

use crate::display_index;

pub type CellIndex = u32;

pub trait CellType: Copy + Debug + PartialEq + Eq + Hash + PartialOrd + Ord {
    /// The type of the constant (pseudo-)cell. This cell type has 2 cells where the cell index
    /// is equivalent to the cell value (i.e. `0` is `false` and `1` is true)
    const CONSTANT: Self;

    /// Number of cells of this type or `None` if infinite amount is available.
    fn count(self) -> Option<CellIndex>;
    fn name(self) -> Cow<'static, str>;
    fn constant(value: bool) -> Cell<Self> {
        Cell::new(Self::CONSTANT, value as CellIndex)
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Cell<CT>(CT, CellIndex);

impl<CT: CellType> Cell<CT> {
    pub fn new(typ: CT, idx: CellIndex) -> Self {
        debug_assert!(
            typ.count().is_none_or(|count| idx < count),
            "cell index {idx} exceeds bounds for type {typ:?}"
        );
        Self(typ, idx)
    }

    /// Returns the value of this cell if it is a constant or `None` if it is not.
    pub fn constant_value(self) -> Option<bool> {
        if self.typ() == CT::CONSTANT {
            Some(self.index() != 0)
        } else {
            None
        }
    }

    pub fn map_type<To: CellType>(self, map: impl FnOnce(CT) -> To) -> Cell<To> {
        Cell::new(map(self.0), self.1)
    }
}

impl<CT> Cell<CT> {
    pub fn index(self) -> CellIndex {
        self.1
    }
    pub fn typ(self) -> CT {
        self.0
    }
}

impl<CT: CellType> Display for Cell<CT> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        if self.0 == CT::CONSTANT {
            write!(f, "{}", self.1 != 0)
        } else {
            write!(f, "{}", self.0.name())?;
            display_index(f, self.1)
        }
    }
}

#[doc(hidden)]
pub fn __display_cell_type<T: CellType>(typ: T, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
    write!(f, "{}", typ.name())
}
