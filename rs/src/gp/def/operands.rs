use std::{
    fmt::{self, Display},
    sync::Arc,
};

use derive_more::{Deref, From};
use derive_where::derive_where;
use itertools::Itertools;

use crate::{gp::def::func, BoolSet};

use super::{Architecture, Cell, CellType, Function, Operand, OperandType};

#[derive_where(Debug, Clone)]
#[derive(Deref, From)]
pub struct OperandTuple<A: Architecture>(
    #[deref(forward)]
    #[from]
    &'static [OperandType<A>],
);

impl<A: Architecture> OperandTuple<A> {
    pub fn try_fit_constants_to_fn(
        &self,
        function: Function,
        value: bool,
    ) -> Option<Vec<Operand<A>>> {
        function.try_compute(value, Some(self.len()), |i, required, preferred| {
            self[i].try_fit_constant(required, preferred)
        })
    }
    pub fn try_fit_constants(&self, function: Function) -> Option<(bool, Vec<Operand<A>>)> {
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

#[derive_where(Debug)]
#[derive(Deref, From)]
pub struct OperandTuples<A: Architecture>(
    #[deref(forward)]
    #[from]
    Vec<OperandTuple<A>>,
);

impl<A: Architecture> OperandTuples<A> {
    pub fn try_fit_constants_to_fn(
        &self,
        function: Function,
        value: bool,
    ) -> Option<Vec<Operand<A>>> {
        self.iter()
            .filter_map(|tuple| tuple.try_fit_constants_to_fn(function, value))
            .next()
    }
    pub fn try_fit_constants(&self, function: Function) -> Option<(bool, Vec<Operand<A>>)> {
        self.iter()
            .filter_map(|tuple| tuple.try_fit_constants(function))
            .next()
    }
}

#[derive_where(Debug, Clone)]
pub struct NaryOperands<A: Architecture>(pub OperandType<A>);

impl<A: Architecture> NaryOperands<A> {
    pub fn try_fit_constants_to_fn(
        &self,
        function: Function,
        value: bool,
    ) -> Option<Vec<Operand<A>>> {
        function.try_compute(value, None, |i, required, preferred| {
            self.0.try_fit_constant(preferred, preferred)
        })
    }
    pub fn try_fit_constants(&self, function: Function) -> Option<(bool, Vec<Operand<A>>)> {
        self.0
            .try_fit_constant(None, None)
            .map(|(value, op)| (value, vec![op]))
    }
}

#[derive_where(Debug, Clone)]
pub enum Operands<A: Architecture> {
    Nary(NaryOperands<A>),
    Tuples(Arc<OperandTuples<A>>),
}

impl<A: Architecture> Operands<A> {
    pub fn nary(typ: OperandType<A>) -> Self {
        Self::Nary(NaryOperands(typ))
    }

    pub fn tuples<'a>(
        tuples: impl IntoIterator<Item = &'static [OperandType<A>]>,
        include: impl IntoIterator<Item = &'a Operands<A>>,
    ) -> Self {
        let mut tuples = tuples.into_iter().map(OperandTuple).collect_vec();
        for operands in include {
            let Operands::Tuples(operands) = operands else {
                panic!("can only expand tuple operands");
            };
            tuples.extend_from_slice(&operands.0);
        }
        Self::Tuples(Arc::new(OperandTuples(tuples)))
    }

    pub fn fit_cell(&self, cell: A::Cell) -> BoolSet {
        match self {
            Self::Tuples(sets) => sets
                .iter()
                .filter(|set| set.len() == 1)
                .map(|set| set[0].fit(cell))
                .collect(),
            Self::Nary(typ) => typ.0.fit(cell),
        }
    }
    pub fn try_fit_constants_to_fn(
        &self,
        function: Function,
        value: bool,
    ) -> Option<Vec<Operand<A>>> {
        match self {
            Self::Nary(typ) => typ.try_fit_constants_to_fn(function, value),
            Self::Tuples(tuples) => tuples.try_fit_constants_to_fn(function, value),
        }
    }
    pub fn try_fit_constants(&self, function: Function) -> Option<(bool, Vec<Operand<A>>)> {
        match self {
            Self::Nary(nary) => nary.try_fit_constants(function),
            Self::Tuples(tuples) => tuples.try_fit_constants(function),
        }
    }
}
