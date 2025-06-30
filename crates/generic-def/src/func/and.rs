use super::EvaluationMethods;
use crate::BoolHint;

pub struct AndEval {
    count: u8,
    value: bool,
}

impl AndEval {
    pub fn new() -> Self {
        Self {
            count: 0,
            value: true,
        }
    }
}

impl EvaluationMethods for AndEval {
    fn hint(&self, arity: Option<usize>, target: bool) -> Option<BoolHint> {
        if target {
            // target is true -> all operands have to be true
            if self.value {
                Some(BoolHint::Require(target))
            } else {
                None
            }
        } else if self.value {
            // target is false and all previous operands were false
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

    fn add(&mut self, value: bool) {
        self.count += 1;
        self.value &= value;
    }

    fn evaluate(&self) -> Option<bool> {
        if self.count == 0 {
            None
        } else {
            Some(self.value)
        }
    }
}

impl Default for AndEval {
    fn default() -> Self {
        Self::new()
    }
}
