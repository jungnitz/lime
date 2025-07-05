use std::{slice, sync::Arc};

use derive_more::{Deref, From};
use itertools::Either;

use crate::{BoolSet, Cell, CellType, OperandType, OperandTypes};

#[derive(Deref, From, Debug, Clone)]
#[deref(forward)]
pub struct OperandTuple<CT>(Vec<OperandType<CT>>);

impl<CT> OperandTuple<CT> {
    pub fn new(operands: Vec<OperandType<CT>>) -> Self {
        Self(operands)
    }
    pub fn as_slice(&self) -> &[OperandType<CT>] {
        self.0.as_slice()
    }
}

#[derive(Debug, Clone, Deref)]
pub struct TupleOperands<CT> {
    arity: usize,
    #[deref(forward)]
    tuples: Arc<[OperandTuple<CT>]>,
}

impl<CT> TupleOperands<CT> {
    pub fn new(tuples: Vec<OperandTuple<CT>>) -> Self {
        let mut iter = tuples.iter();
        let arity = iter
            .next()
            .expect("at least one tuple has to be present")
            .len();
        for tuple in iter {
            assert_eq!(tuple.len(), arity, "tuple lengths do not match");
        }
        Self {
            arity,
            tuples: tuples.into(),
        }
    }

    pub fn arity(&self) -> usize {
        self.arity
    }
}

impl<CT: CellType> TupleOperands<CT> {
    pub fn fit(&self, cell: Cell<CT>) -> BoolSet {
        self.tuples
            .iter()
            .filter(|set| set.len() == 1)
            .map(|set| set[0].fit(cell))
            .collect()
    }
}

#[derive(Debug, Clone)]
pub struct NaryOperands<CT>(pub OperandTypes<CT>);

#[derive(Debug, Clone)]
pub enum Operands<CT> {
    Nary(NaryOperands<CT>),
    Tuples(TupleOperands<CT>),
}

impl<CT> Operands<CT> {
    /// Returns the number of decribed operands or `None` if the number is variable.
    pub fn arity(&self) -> Option<usize> {
        match self {
            Self::Nary(_) => None,
            Self::Tuples(tuples) => Some(tuples.arity()),
        }
    }
}

impl<CT: CellType> Operands<CT> {
    /// Returns all combinations of operands that fit this description. For descriptions of n-ary
    /// operands returns only a minimal set of combinations (i.e. slices of length 1).
    pub fn combinations(&self) -> impl Iterator<Item = &[OperandType<CT>]> {
        match self {
            Self::Tuples(tuples) => Either::Left(tuples.iter().map(|tuple| tuple.as_slice())),
            Self::Nary(nary) => Either::Right(nary.0.iter().map(slice::from_ref)),
        }
    }

    /// Returns the inverted-values for which using **only** the given cell for the described
    /// operands described is valid
    pub fn fit_cell(&self, cell: Cell<CT>) -> BoolSet {
        match self {
            Self::Tuples(tuples) => tuples.fit(cell),
            Self::Nary(typ) => typ.0.fit(cell),
        }
    }
}
