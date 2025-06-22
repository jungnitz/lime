#![allow(unused)]

mod dsl;
mod ops;
mod program;
mod state;

use std::fmt::Debug;
use std::hash::Hash;

use derive_more::{Deref, From};

use crate::BoolSet;

pub type CellIndex = u32;

/// Abstractly describes a Logic-in-Memory architecture.
pub trait Architecture: Sized + 'static {
    type CellType: CellType<Arch = Self>;
    type Cell: Cell<Arch = Self>;

    fn outputs(&self) -> Operands<Self>;
    fn operations(&self) -> &'static [OperationType<Self>];

    fn cell(&self, typ: Self::CellType, idx: CellIndex) -> Self::Cell;
    fn constant(&self, value: bool) -> Self::Cell {
        Self::cell(self, Self::CellType::CONSTANT, value as u32)
    }
}

pub trait CellType: Copy + Debug + PartialEq + Eq + Hash {
    type Arch: Architecture<Cell = Self>;
    const CONSTANT: Self;

    /// Number of cells of this type or `None` if infinite amount is available.
    fn count(self) -> Option<CellIndex>;
}

pub trait Cell: Copy + Debug + PartialEq + Eq + Hash {
    type Arch: Architecture<Cell = Self>;

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

pub struct OperandType<A: Architecture> {
    typ: A::CellType,
    inverted: bool,
    index: Option<CellIndex>,
}

impl<A: Architecture> OperandType<A> {
    /// Checks whether the given `cell` fits this operand type. If it does, returns whether the
    /// inverted value fits this operand (i.e. is used for functions of the operation).
    pub fn fit(&self, cell: A::Cell) -> Option<bool> {
        if self.typ == cell.typ() && self.index.is_none_or(|i| i == cell.index()) {
            Some(self.inverted)
        } else {
            None
        }
    }
}

#[derive(Deref, From)]
pub struct OperandSet<A: Architecture>(
    #[deref]
    #[from]
    &'static [OperandType<A>],
);

impl<A: Architecture> OperandSet<A> {
    pub fn positions(&self, cell: A::Cell) -> impl Iterator<Item = (usize, bool)> {
        self.0
            .iter()
            .enumerate()
            .filter_map(move |(i, typ)| typ.fit(cell).map(|inv| (i, inv)))
    }
}

pub enum Operands<A: Architecture> {
    Nary(OperandType<A>),
    Sets(&'static [OperandSet<A>]),
}

impl<A: Architecture> Operands<A> {
    pub fn fit_cell(&self, cell: A::Cell) -> BoolSet {
        match self {
            Self::Sets(sets) => sets
                .iter()
                .filter(|set| set.len() == 1)
                .filter_map(|set| set[0].fit(cell))
                .fold(BoolSet::None, |set, inv| set.insert(inv)),
            Self::Nary(typ) => BoolSet::None.inser_optional(typ.fit(cell)),
        }
    }
}

pub struct OperationType<A: Architecture> {
    pub input: Operands<A>,
    pub input_results: Vec<InputResult>,
    pub output: Option<Function>,
}

pub struct Operation<A: Architecture> {
    pub typ: OperationType<A>,
    pub inputs: Vec<A::Cell>,
    pub outputs: Vec<A::Cell>,
}

pub struct Function {
    pub inverted: bool,
    pub gate: Gate,
}

impl Function {
    pub fn try_compute<R>(
        &self,
        target: bool,
        arity: Option<usize>,
        mut candidate_fn: impl FnMut(usize, Option<bool>, Option<bool>) -> Option<(bool, R)>,
    ) -> Option<Vec<R>> {
        let target = target ^ self.inverted;
        let mut results = Vec::new();
        let mut candidate_fn = |i, required, preferred| {
            let (value, r) = candidate_fn(i, required, preferred)?;
            debug_assert!(
                required.is_none_or(|required| value == required),
                "candidate_fn did not return required value"
            );
            Some((value, r))
        };

        // 0..arity or 0..
        let is = {
            let a = arity.map(|arity| 0..arity).into_iter().flatten();
            let b = match arity {
                Some(_) => None,
                None => Some(0..),
            }
            .into_iter()
            .flatten();
            a.chain(b)
        };

        match self.gate {
            Gate::And => {
                let mut current_value = true;
                for i in is {
                    let required = if target {
                        Some(true)
                    } else if Some(i + 1) == arity && current_value {
                        Some(false)
                    } else {
                        None
                    };
                    let (value, r) = candidate_fn(i, required, Some(target))?;
                    results.push(r);
                    current_value &= value;
                    debug_assert!(!target || current_value, "AND no longer fulfillable");
                    if current_value == target && arity.is_none() {
                        return Some(results);
                    }
                }
            }
            Gate::Maj => {
                debug_assert!(
                    arity.is_none_or(|arity| arity % 2 == 1),
                    "MAJ has to have uneven arity"
                );
                let mut num_target = 0;
                for i in is {
                    let required = if let Some(arity) = arity {
                        let missing_values = arity / 2 - num_target;
                        let leftover = arity - i;
                        debug_assert!(leftover >= missing_values);
                        if missing_values == leftover {
                            Some(target)
                        } else {
                            None
                        }
                    } else {
                        None
                    };
                    let (value, r) = candidate_fn(i, required, Some(target))?;
                    results.push(r);
                    if value == target {
                        num_target += 1;
                    }
                    if arity.is_none() && i % 2 == 1 && num_target > i / 2 {
                        return Some(results);
                    }
                }
            }
            Gate::Constant(c) => {
                if c != target {
                    return None;
                }
                let n = arity.unwrap_or(0);
                for i in 0..n {
                    results.push(candidate_fn(i, None, None)?.1);
                }
            }
        }
        Some(results)
    }
}

pub enum Gate {
    And,
    Maj,
    Constant(bool),
}

pub enum InputResult {
    Unchanged,
    Destroyed,
    Function(Function),
}
