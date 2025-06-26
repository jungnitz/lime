use std::{fmt::Display, marker::PhantomData};

use derive_where::derive_where;

use crate::{gp::def::display_maybe_inverted, BoolSet};

use super::{Architecture, Cell, CellIndex, CellType};

#[derive_where(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Operand<A: Architecture> {
    pub cell: A::Cell,
    pub inverted: bool,
}

impl<A> Display for Operand<A>
where
    A: Architecture,
    A::Cell: Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        display_maybe_inverted(f, self.cell, self.inverted)
    }
}

#[derive_where(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OperandType<A: Architecture> {
    pub typ: A::CellType,
    pub inverted: bool,
    pub index: Option<CellIndex>,
}

impl<A> Display for OperandType<A>
where
    A: Architecture,
    A::CellType: Display,
    A::Cell: Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.index {
            None => display_maybe_inverted(f, self.typ, self.inverted),
            Some(i) => display_maybe_inverted(f, A::cell(self.typ, i), self.inverted),
        }
    }
}

impl<A: Architecture> OperandType<A> {
    /// Checks whether the given `cell` can be used for an operand of this type. If it can, returns
    /// whether the operand must/can reference the inverted cell value.
    pub fn fit(&self, cell: A::Cell) -> BoolSet {
        if self.typ == cell.typ() && self.index.is_none_or(|i| i == cell.index()) {
            BoolSet::from(self.inverted)
        } else {
            BoolSet::None
        }
    }

    /// Attempts to find a constant value operand that fits this operand type and so that
    /// * if `required` is given, the operand has its value
    /// * else if `preferred` is given and this operand matches both constants, the operand has the
    ///   preferred value
    /// Returns the matching operand together with its value.
    pub fn try_fit_constant(
        &self,
        mut required: Option<bool>,
        mut preferred: Option<bool>,
    ) -> Option<(bool, Operand<A>)> {
        if self.typ != <A::CellType as CellType>::CONSTANT {
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
                    cell: A::constant(value),
                    inverted: self.inverted,
                },
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use itertools::Itertools;

    use crate::gp::{Ambit, AmbitCell, AmbitCellType};

    use super::*;

    #[test]
    fn fit() {
        assert_eq!(
            OperandType::<Ambit> {
                typ: AmbitCellType::DCC,
                inverted: true,
                index: Some(10),
            }
            .fit(AmbitCell::DCC(10)),
            BoolSet::Single(true)
        );
        assert_eq!(
            OperandType::<Ambit> {
                typ: AmbitCellType::T,
                inverted: false,
                index: Some(1),
            }
            .fit(AmbitCell::T(1)),
            BoolSet::Single(false)
        );
        assert_eq!(
            OperandType::<Ambit> {
                typ: AmbitCellType::DCC,
                inverted: true,
                index: Some(0),
            }
            .fit(AmbitCell::DCC(1)),
            BoolSet::None
        );
        assert_eq!(
            OperandType::<Ambit> {
                typ: AmbitCellType::D,
                inverted: true,
                index: Some(0),
            }
            .fit(AmbitCell::DCC(0)),
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
                let typ = OperandType::<Ambit> {
                    typ: AmbitCellType::Constant,
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
