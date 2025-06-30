use crate::{BoolHint, func::EvaluationMethods};

pub struct MajEval {
    nums: [u8; 2],
}

impl MajEval {
    pub fn new() -> Self {
        Self { nums: [0, 0] }
    }

    fn count(&self) -> usize {
        (self.nums[0] + self.nums[1]).into()
    }
}

impl EvaluationMethods for MajEval {
    fn hint(&self, arity: Option<usize>, target: bool) -> Option<BoolHint> {
        let Some(arity) = arity else {
            return Some(BoolHint::Prefer(target));
        };
        let num_target = usize::from(self.nums[target as usize]);
        let required_target = arity.div_ceil(2);
        if num_target >= required_target {
            return Some(BoolHint::Any);
        }
        let missing_values = required_target - num_target;
        let leftover = arity - self.count();
        if leftover < missing_values {
            None
        } else if missing_values == leftover {
            Some(BoolHint::Require(target))
        } else {
            Some(BoolHint::Prefer(target))
        }
    }

    fn add(&mut self, value: bool) {
        self.nums[value as usize] += 1
    }

    fn evaluate(&self) -> Option<bool> {
        if self.count() % 2 != 1 {
            None
        } else {
            Some(self.nums[1] > self.nums[0])
        }
    }
}

impl Default for MajEval {
    fn default() -> Self {
        Self::new()
    }
}
