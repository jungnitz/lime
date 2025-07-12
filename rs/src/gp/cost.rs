use std::ops::Add;

use lime_generic_def::Operation;

use crate::gp::CellOrVar;

pub trait OperationCost<CT> {
    type Cost: PartialOrd + Ord + Clone + Add<Output = Self::Cost>;

    fn cost<I: Into<CellOrVar<CT>>>(&self, operation: &Operation<I, CT>) -> Self::Cost;
}

pub struct EqualCosts;

impl<CT> OperationCost<CT> for EqualCosts {
    type Cost = u64;

    fn cost<I: Into<CellOrVar<CT>>>(&self, operation: &Operation<I, CT>) -> Self::Cost {
        1
    }
}
