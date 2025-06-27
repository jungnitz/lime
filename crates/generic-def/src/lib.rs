mod boolset;
mod cell;
mod func;
mod operand;
mod operands;
mod operation;
mod outputs;

use std::fmt::{Display, Formatter};

pub use self::{
    boolset::BoolSet, cell::*, func::*, operand::*, operands::*, operation::*, outputs::*,
};

/// Abstractly describes a Logic-in-Memory architecture.
pub struct Architecture<CT> {
    operations: Operations<CT>,
    outputs: Outputs<CT>,
}

impl<CT> Architecture<CT> {
    pub fn new(operations: Operations<CT>, outputs: Outputs<CT>) -> Self {
        Self {
            operations,
            outputs,
        }
    }
}

impl<CT> Architecture<CT> {
    pub fn outputs(&self) -> &Outputs<CT> {
        &self.outputs
    }
    pub fn operations(&self) -> &Operations<CT> {
        &self.operations
    }
}

fn display_maybe_inverted(f: &mut Formatter<'_>, inverted: bool) -> std::fmt::Result {
    if inverted { write!(f, "!") } else { Ok(()) }
}

fn display_index<D: Display>(f: &mut Formatter<'_>, idx: D) -> std::fmt::Result {
    write!(f, "[{idx}]")
}

fn display_opt_index<D: Display>(f: &mut Formatter<'_>, idx: Option<D>) -> std::fmt::Result {
    if let Some(idx) = idx {
        display_index(f, idx)
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
    pub enum DummyCellType {
        Constant,
        A,
        B,
    }

    impl CellType for DummyCellType {
        const CONSTANT: Self = Self::Constant;

        fn count(self) -> Option<CellIndex> {
            match self {
                Self::Constant => Some(2),
                Self::A => Some(4),
                Self::B => None,
            }
        }

        fn name(self) -> &'static str {
            match self {
                Self::Constant => "bool",
                Self::A => "A",
                Self::B => "B",
            }
        }
    }
}
