use std::sync::Arc;

use derive_more::{Deref, From};

use crate::{BoolSet, Cell, CellType, Function, Operand, OperandType};

#[derive(Deref, From, Debug, Clone)]
#[deref(forward)]
pub struct OperandTuple<CT>(Vec<OperandType<CT>>);

impl<CT> OperandTuple<CT> {
    pub fn new(operands: Vec<OperandType<CT>>) -> Self {
        Self(operands)
    }
}

impl<CT: CellType> OperandTuple<CT> {
    pub fn try_fit_constants_to_fn(
        &self,
        function: Function,
        value: bool,
    ) -> Option<Vec<Operand<CT>>> {
        function.try_compute(value, Some(self.len()), |i, required, preferred| {
            self[i].try_fit_constant(required, preferred)
        })
    }
    pub fn try_fit_constants(&self, function: Function) -> Option<(bool, Vec<Operand<CT>>)> {
        let mut result = Vec::with_capacity(self.len());
        let mut eval = function.evaluate();
        for typ in self.iter() {
            let (value, op) = typ.try_fit_constant(None, None)?;
            result.push(op);
            eval.add(value);
        }
        Some((eval.get(), result))
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
    pub fn try_fit_constants_to_fn(
        &self,
        function: Function,
        value: bool,
    ) -> Option<Vec<Operand<CT>>> {
        self.tuples
            .iter()
            .filter_map(|tuple| tuple.try_fit_constants_to_fn(function, value))
            .next()
    }
    pub fn try_fit_constants(&self, function: Function) -> Option<(bool, Vec<Operand<CT>>)> {
        self.tuples
            .iter()
            .filter_map(|tuple| tuple.try_fit_constants(function))
            .next()
    }
}

#[derive(Debug, Clone)]
pub struct NaryOperands<CT>(pub OperandType<CT>);

impl<CT: CellType> NaryOperands<CT> {
    pub fn try_fit_constants_to_fn(
        &self,
        function: Function,
        value: bool,
    ) -> Option<Vec<Operand<CT>>> {
        function.try_compute(value, None, |_, _, preferred| {
            self.0.try_fit_constant(preferred, preferred)
        })
    }
    pub fn try_fit_constants(&self, function: Function) -> Option<(bool, Vec<Operand<CT>>)> {
        self.0.try_fit_constant(None, None).map(|(value, op)| {
            let mut eval = function.evaluate();
            eval.add(value);
            (eval.get(), vec![op])
        })
    }
}

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
    /// Returns the inverted-values for which using **only** the given cell for the described
    /// operands described is valid
    pub fn fit_cell(&self, cell: Cell<CT>) -> BoolSet {
        match self {
            Self::Tuples(tuples) => tuples.fit(cell),
            Self::Nary(typ) => typ.0.fit(cell),
        }
    }

    /// Attempt to use only constants for the described operands and so that the result of the given
    /// function with the selected operands is the target `value`.
    /// Returns the matched operands on success, or `None` if not possible.
    pub fn try_fit_constants_to_fn(
        &self,
        function: Function,
        value: bool,
    ) -> Option<Vec<Operand<CT>>> {
        match self {
            Self::Nary(typ) => typ.try_fit_constants_to_fn(function, value),
            Self::Tuples(tuples) => tuples.try_fit_constants_to_fn(function, value),
        }
    }

    /// Attempt to use only constants for the described operands.
    /// Returns the value of the given function and the selected operands on success or `None` if
    /// not possible.
    pub fn try_fit_constants(&self, function: Function) -> Option<(bool, Vec<Operand<CT>>)> {
        match self {
            Self::Nary(nary) => nary.try_fit_constants(function),
            Self::Tuples(tuples) => tuples.try_fit_constants(function),
        }
    }
}
