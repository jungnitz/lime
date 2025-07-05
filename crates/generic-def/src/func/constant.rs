use super::EvaluationMethods;
use crate::BoolHint;

pub struct ConstEval {
    const_value: bool,
}

impl ConstEval {
    pub fn new(const_value: bool) -> Self {
        Self { const_value }
    }
}

impl EvaluationMethods for ConstEval {
    fn hint(&self, _arity: Option<usize>, target: bool) -> Option<BoolHint> {
        if self.const_value == target {
            Some(BoolHint::Any)
        } else {
            None
        }
    }

    fn hint_to_ident(&self, _arity: Option<usize>, _inverted: bool) -> Option<BoolHint> {
        None
    }

    fn add(&mut self, _value: bool) {}

    fn evaluate(&self) -> Option<bool> {
        Some(self.const_value)
    }
}
