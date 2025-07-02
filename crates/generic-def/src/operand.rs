use std::fmt::Display;

use derive_more::{Deref, From};

use crate::{
    BoolHint, BoolSet, Cell, CellIndex, CellType, display_maybe_inverted, display_opt_index,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Operand<CT> {
    pub cell: Cell<CT>,
    pub inverted: bool,
}

impl<CT> Display for Operand<CT>
where
    CT: Display + CellType,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        display_maybe_inverted(f, self.inverted)?;
        write!(f, "{}", self.cell)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OperandType<CT> {
    pub typ: CT,
    pub inverted: bool,
    pub index: Option<CellIndex>,
}

impl<CT> Display for OperandType<CT>
where
    CT: Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        display_maybe_inverted(f, self.inverted)?;
        write!(f, "{}", self.typ)?;
        display_opt_index(f, self.index)
    }
}

impl<CT: CellType> OperandType<CT> {
    /// Checks whether the given `cell` can be used for an operand of this type. If it can, returns
    /// whether the operand must reference the inverted cell value.
    pub fn fit(&self, cell: Cell<CT>) -> Option<bool> {
        if self.typ == cell.typ() && self.index.is_none_or(|i| i == cell.index()) {
            Some(self.inverted)
        } else {
            None
        }
    }

    /// Attempts to find a constant value operand that fits this operand type and so that
    ///
    /// * if `required` is given, the operand has its value
    /// * else if `preferred` is given and this operand matches both constants, the operand has the
    ///   preferred value
    ///
    /// Returns the matching operand together with its value.
    pub fn try_fit_constant(&self, mut hint: BoolHint) -> Option<(bool, Operand<CT>)> {
        if self.typ != CT::CONSTANT {
            return None;
        }
        hint = hint.map(|v| v ^ self.inverted);
        match (self.index, hint) {
            (None, BoolHint::Require(v)) | (None, BoolHint::Prefer(v)) => Some(v),
            (None, BoolHint::Any) => Some(true),
            (Some(i), BoolHint::Require(required)) => {
                if required == (i != 0) {
                    Some(required)
                } else {
                    None
                }
            }
            (Some(i), _) => Some(i != 0),
        }
        .map(|value| {
            (
                value ^ self.inverted,
                Operand {
                    cell: CT::constant(value),
                    inverted: self.inverted,
                },
            )
        })
    }
}

#[derive(Deref, From, Debug, Clone)]
#[deref(forward)]
pub struct OperandTypes<CT>(Vec<OperandType<CT>>);

impl<CT: CellType> OperandTypes<CT> {
    pub fn fit(&self, cell: Cell<CT>) -> BoolSet {
        self.iter().map(|op| op.fit(cell)).collect()
    }
    pub fn try_fit_constant(&self, hint: BoolHint) -> Option<(bool, Operand<CT>)> {
        self.iter()
            .filter_map(|op| op.try_fit_constant(hint))
            .next()
    }
}

#[cfg(test)]
mod tests {
    use itertools::Itertools;

    use crate::tests::DummyCellType;

    use super::*;

    #[test]
    fn fit() {
        assert_eq!(
            OperandType {
                typ: DummyCellType::B,
                inverted: true,
                index: Some(10),
            }
            .fit(Cell::new(DummyCellType::B, 10)),
            Some(true)
        );
        assert_eq!(
            OperandType {
                typ: DummyCellType::A,
                inverted: false,
                index: Some(1),
            }
            .fit(Cell::new(DummyCellType::A, 1)),
            Some(false)
        );
        assert_eq!(
            OperandType {
                typ: DummyCellType::B,
                inverted: true,
                index: Some(0),
            }
            .fit(Cell::new(DummyCellType::B, 1)),
            None
        );
        assert_eq!(
            OperandType {
                typ: DummyCellType::A,
                inverted: true,
                index: Some(0),
            }
            .fit(Cell::new(DummyCellType::B, 0)),
            None
        );
    }

    #[test]
    pub fn try_fit_constant() {
        for inverted in [true, false] {
            for (cell_value, required, preferred) in [None, Some(true), Some(false)]
                .into_iter()
                .tuple_combinations()
            {
                let typ = OperandType {
                    typ: DummyCellType::Constant,
                    inverted,
                    index: cell_value.map(|i| i as CellIndex),
                };
                let hint = match (required, preferred) {
                    (Some(required), _) => BoolHint::Require(required),
                    (None, Some(preferred)) => BoolHint::Prefer(preferred),
                    (None, None) => BoolHint::Any,
                };
                let result = typ.try_fit_constant(hint);
                println!("{typ:#?} {hint:?} {result:?}");
                let possible = match (required, cell_value) {
                    (Some(required), Some(cell_value)) => required == cell_value ^ inverted,
                    _ => true,
                };
                if let Some((value, operand)) = result {
                    assert!(possible, "should not fit");
                    let operand_value =
                        operand.cell.constant_value().expect("should be a constant")
                            ^ operand.inverted;
                    assert_eq!(value, operand_value, "value and operand_value do not match");
                    assert!(
                        required.is_none_or(|required| value == required),
                        "value is not the required value"
                    );
                    assert!(
                        typ.fit(operand.cell)
                            .is_none_or(|inverted| inverted == operand.inverted),
                        "returned operand does not fit operand type"
                    );
                    if let (None, Some(preferred)) = (required, preferred) {
                        let can_be_preferred =
                            cell_value.is_none_or(|cell_value| preferred == cell_value ^ inverted);
                        assert_eq!(
                            can_be_preferred,
                            preferred == value,
                            "should be preferred value"
                        );
                    }
                } else {
                    assert!(!possible, "should fit");
                }
            }
        }
    }
}
