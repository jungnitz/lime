use super::EvaluationMethods;
use crate::BoolHint;

pub struct AndEval {
    count: u8,
    value: Option<bool>,
}

impl AndEval {
    pub fn new() -> Self {
        Self {
            count: 0,
            value: Some(true),
        }
    }
}

impl EvaluationMethods for AndEval {
    fn hint(&self, arity: Option<usize>, target: bool) -> Option<BoolHint> {
        if target {
            // target is true -> all operands have to be true
            if let Some(value) = self.value
                && value
            {
                Some(BoolHint::Require(target))
            } else {
                None
            }
        } else if self.value.unwrap_or(true) {
            // target is false and all previous operands were true or unknown
            if Some(usize::from(self.count) + 1) == arity {
                // only one left, this needs to flip the value to false
                Some(BoolHint::Require(false))
            } else {
                // false would lead to the rest of the operands to be freely choosable
                Some(BoolHint::Prefer(false))
            }
        } else {
            // target is false and value is already false, next values can be whatever
            Some(BoolHint::Any)
        }
    }

    fn hint_id(&self, arity: Option<usize>, inverted: Option<bool>) -> Option<BoolHint> {
        if inverted == Some(true) {
            return None;
        }
        let Some(value) = self.value else {
            return None;
        };
        if !value {
            None
        } else if let Some(arity) = arity {
            if self.count + 1 == arity as u8 {
                Some(BoolHint::Any)
            } else {
                Some(BoolHint::Require(true))
            }
        } else {
            Some(BoolHint::Any)
        }
    }

    fn id_inverted(&self) -> Option<bool> {
        if self.value == Some(true) {
            Some(false)
        } else {
            None
        }
    }

    fn add(&mut self, value: bool) {
        self.count += 1;
        if !value {
            self.value = Some(false);
        }
    }

    fn add_unknown(&mut self) {
        self.value = None;
    }

    fn evaluate(&self) -> Option<bool> {
        if self.count == 0 { None } else { self.value }
    }
}

impl Default for AndEval {
    fn default() -> Self {
        Self::new()
    }
}
