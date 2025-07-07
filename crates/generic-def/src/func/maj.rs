use crate::{BoolHint, func::EvaluationMethods};

pub struct MajEval {
    nums: [u8; 3],
}

impl MajEval {
    pub fn new() -> Self {
        Self { nums: [0, 0, 0] }
    }

    fn count(&self) -> usize {
        self.nums.iter().sum::<u8>().into()
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

    fn hint_id(&self, arity: Option<usize>, inverted: Option<bool>) -> Option<BoolHint> {
        if inverted == Some(true) || self.nums[2] != 0 {
            return None;
        }
        if self.nums[0] == self.nums[1] {
            Some(BoolHint::Any)
        } else if let Some(arity) = arity {
            if arity == self.count() + 1 {
                // next value is last value, and we do not have equal counts
                // -> impossible
                None
            } else {
                let leftover = arity - self.count() - 1;
                // delta: number of min-values to reach equal counts
                let (delta, min) = if self.nums[0] > self.nums[1] {
                    (self.nums[0] - self.nums[1], true)
                } else {
                    (self.nums[1] - self.nums[0], false)
                };
                let delta = delta as usize;
                if leftover < delta {
                    None
                } else if leftover == delta {
                    Some(BoolHint::Require(min))
                } else {
                    Some(BoolHint::Prefer(min))
                }
            }
        } else if self.nums[0] > self.nums[1] {
            Some(BoolHint::Prefer(true))
        } else {
            Some(BoolHint::Prefer(false))
        }
    }

    fn id_inverted(&self) -> Option<bool> {
        if self.nums[0] == self.nums[1] && self.nums[2] == 0 {
            Some(false)
        } else {
            None
        }
    }

    fn add(&mut self, value: bool) {
        self.nums[value as usize] += 1
    }

    fn add_unknown(&mut self) {
        self.nums[2] += 1;
    }

    fn evaluate(&self) -> Option<bool> {
        if self.count() % 2 != 1 {
            None
        } else {
            let value = self.nums[1] > self.nums[0];
            let diff = self.nums[value as usize] - self.nums[!value as usize];
            if diff <= self.nums[2] {
                None
            } else {
                Some(value)
            }
        }
    }
}

impl Default for MajEval {
    fn default() -> Self {
        Self::new()
    }
}
