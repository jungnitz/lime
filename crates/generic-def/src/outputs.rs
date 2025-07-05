use std::sync::Arc;

use derive_more::Deref;

use crate::{BoolSet, Cell, CellType, Operands};

#[derive(Deref)]
#[deref(forward)]
pub struct Outputs<CT>(Arc<[Operands<CT>]>);

impl<CT> Outputs<CT> {
    pub fn contains_none(&self) -> bool {
        self.0.is_empty() || self.iter().any(|operands| operands.arity() == Some(0))
    }
    pub fn new(vec: Vec<Operands<CT>>) -> Self {
        Self(vec.into())
    }
}

impl<CT: CellType> Outputs<CT> {
    /// See: [Operands::fit_cell]
    pub fn fit_cell(&self, cell: Cell<CT>) -> BoolSet {
        self.iter().map(|ops| ops.fit_cell(cell)).collect()
    }
}
