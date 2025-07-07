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

    fn hint_id(&self, _arity: Option<usize>, _inverted: Option<bool>) -> Option<BoolHint> {
        None
    }

    fn id_inverted(&self) -> Option<bool> {
        None
    }

    fn add(&mut self, _value: bool) {}

    fn add_unknown(&mut self) {}

    fn evaluate(&self) -> Option<bool> {
        Some(self.const_value)
    }
}
