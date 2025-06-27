use std::fmt::Display;

use crate::{BoolSet, display_maybe_inverted, display_opt_index};

use super::{Cell, CellIndex, CellType};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Operand<CT> {
    pub cell: Cell<CT>,
    pub inverted: bool,
}

impl<CT> Display for Operand<CT>
where
    CT: Display,
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
    /// whether the operand must/can reference the inverted cell value.
    pub fn fit(&self, cell: Cell<CT>) -> BoolSet {
        if self.typ == cell.typ() && self.index.is_none_or(|i| i == cell.index()) {
            BoolSet::from(self.inverted)
        } else {
            BoolSet::None
        }
    }

    /// Attempts to find a constant value operand that fits this operand type and so that
    ///
    /// * if `required` is given, the operand has its value
    /// * else if `preferred` is given and this operand matches both constants, the operand has the
    ///   preferred value
    ///
    /// Returns the matching operand together with its value.
    pub fn try_fit_constant(
        &self,
        mut required: Option<bool>,
        mut preferred: Option<bool>,
    ) -> Option<(bool, Operand<CT>)> {
        if self.typ != CT::CONSTANT {
            return None;
        }
        required = required.map(|required| required ^ self.inverted);
        preferred = preferred.map(|preferred| preferred ^ self.inverted);
        match self.index {
            None => Some(required.or(preferred).unwrap_or(true)),
            Some(i) => {
                let cell_value = i != 0;
                match required {
                    None => Some(cell_value),
                    Some(required) => {
                        if required == cell_value {
                            Some(cell_value)
                        } else {
                            None
                        }
                    }
                }
            }
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
            BoolSet::Single(true)
        );
        assert_eq!(
            OperandType {
                typ: DummyCellType::A,
                inverted: false,
                index: Some(1),
            }
            .fit(Cell::new(DummyCellType::A, 1)),
            BoolSet::Single(false)
        );
        assert_eq!(
            OperandType {
                typ: DummyCellType::B,
                inverted: true,
                index: Some(0),
            }
            .fit(Cell::new(DummyCellType::B, 1)),
            BoolSet::None
        );
        assert_eq!(
            OperandType {
                typ: DummyCellType::A,
                inverted: true,
                index: Some(0),
            }
            .fit(Cell::new(DummyCellType::B, 0)),
            BoolSet::None
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
                let result = typ.try_fit_constant(required, preferred);
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
                        typ.fit(operand.cell).contains(operand.inverted),
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
