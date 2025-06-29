use std::fmt::Display;

use strum::EnumString;

use crate::display_maybe_inverted;

// Gate type without input/output inverters
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, EnumString)]
#[strum(ascii_case_insensitive)]
pub enum Gate {
    And,
    Maj,
    #[strum(disabled)]
    Constant(bool),
}

impl Gate {
    /// Try to find a set of input values so that the value of this gate is the given target
    /// value.
    ///
    /// The candidate function is called once for each input value index or, if no arity is given,
    /// until the target value is reached and has the following arguments:
    ///
    /// - The current input argument index (in `0..arity` or `0..`) and for the input at that index
    /// - the value that is **required** in order for the gate to have the target value. If a
    ///   different value is returned, this function returns `None`.
    /// - the value that is **preferred**. Returning such a value will potentially result in more
    ///   freedom for later inputs
    ///
    /// The return value of the candidate function indicates the chosen value for this input as the
    /// first value. The second value is collected into the returned `Vec` (in order of the input
    /// argument indices). When returning `None` from the candidate function at any time, this means
    /// that no candidate was found and therefore this function returns `None` as well.
    pub fn try_compute<R>(
        self,
        target: bool,
        arity: Option<usize>,
        mut candidate_fn: impl FnMut(usize, Option<bool>, Option<bool>) -> Option<(bool, R)>,
    ) -> Option<Vec<R>> {
        let mut results = match arity {
            Some(arity) => Vec::with_capacity(arity),
            None => Vec::new(),
        };
        let mut candidate_fn = |i, required, preferred| {
            let (value, r) = candidate_fn(i, required, preferred)?;
            if required.is_none_or(|required| value == required) {
                Some((value, r))
            } else {
                None
            }
        };

        // 0..arity or 0..
        let is = {
            let a = arity.map(|arity| 0..arity).into_iter().flatten();
            let b = match arity {
                Some(_) => None,
                None => Some(0..),
            }
            .into_iter()
            .flatten();
            a.chain(b)
        };

        match self {
            Gate::And => {
                let mut current_value = true;
                for i in is {
                    let required = if target {
                        Some(true)
                    } else if Some(i + 1) == arity && current_value {
                        Some(false)
                    } else {
                        None
                    };
                    let preferred = if !target && !current_value {
                        None
                    } else {
                        Some(target)
                    };
                    let (value, r) = candidate_fn(i, required, preferred)?;
                    results.push(r);
                    current_value &= value;
                    debug_assert!(!target || current_value, "AND no longer fulfillable");
                    if current_value == target && arity.is_none() {
                        return Some(results);
                    }
                }
            }
            Gate::Maj => {
                debug_assert!(
                    arity.is_none_or(|arity| arity % 2 == 1),
                    "MAJ has to have uneven arity"
                );
                let mut num_target = 0;
                for i in is {
                    let (required, preferred) = if let Some(arity) = arity {
                        let required_target = arity.div_ceil(2);
                        if num_target >= required_target {
                            (None, None)
                        } else {
                            let missing_values = required_target - num_target;
                            let leftover = arity - i;
                            debug_assert!(leftover >= missing_values, "MAJ no longer fullfillable");
                            if missing_values == leftover {
                                (Some(target), Some(target))
                            } else {
                                (None, Some(target))
                            }
                        }
                    } else {
                        (None, Some(target))
                    };
                    let (value, r) = candidate_fn(i, required, preferred)?;
                    results.push(r);
                    if value == target {
                        num_target += 1;
                    }
                    if arity.is_none() && i % 2 == 0 && num_target > i / 2 {
                        return Some(results);
                    }
                }
            }
            Gate::Constant(c) => {
                if c != target {
                    return None;
                }
                let n = arity.unwrap_or(0);
                for i in 0..n {
                    results.push(candidate_fn(i, None, None)?.1);
                }
            }
        }
        Some(results)
    }
}

impl Display for Gate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::And => write!(f, "and"),
            Self::Maj => write!(f, "maj"),
            Self::Constant(c) => write!(f, "{c:?}"),
        }
    }
}

/// Gate with an optional output inverter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Function {
    pub inverted: bool,
    pub gate: Gate,
}

impl Function {
    /// Shortcut for `self.gate.`[`try_compute`](Gate::try_compute)`(target ^ self.inverted, arity, candidate_fn)`
    pub fn try_compute<R>(
        &self,
        target: bool,
        arity: Option<usize>,
        candidate_fn: impl FnMut(usize, Option<bool>, Option<bool>) -> Option<(bool, R)>,
    ) -> Option<Vec<R>> {
        self.gate
            .try_compute(target ^ self.inverted, arity, candidate_fn)
    }
    pub fn evaluate(self) -> Evaluation {
        Evaluation {
            function: self,
            num: [0, 0],
        }
    }
}

impl Display for Function {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        display_maybe_inverted(f, self.inverted)?;
        write!(f, "{}", self.gate)
    }
}

pub struct Evaluation {
    function: Function,
    num: [u16; 2],
}

impl Evaluation {
    pub fn add(&mut self, value: bool) {
        self.num[value as usize] += 1;
    }
    pub fn get(self) -> bool {
        let total = self.num[0] + self.num[1];
        let gate_value = match self.function.gate {
            Gate::Constant(c) => c,
            Gate::And => {
                assert!(total > 0, "no value for AND given");
                self.num[0] == 0 // no falses
            }
            Gate::Maj => {
                assert!(
                    total % 2 == 1,
                    "even number of arguments to MAJ given ({total})"
                );
                self.num[1] > self.num[0]
            }
        };
        gate_value ^ self.function.inverted
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn evaluate_maj() {
        for (values, result) in [
            (&[true] as &[bool], true),
            (&[true, false, true], true),
            (&[true, false, false], false),
            (&[true, false, true, false, false], false),
        ] {
            let mut eval = Function {
                gate: Gate::Maj,
                inverted: false,
            }
            .evaluate();
            values.iter().for_each(|value| eval.add(*value));
            assert_eq!(eval.get(), result, "invalid result")
        }
    }

    #[test]
    pub fn evaluate_and() {
        for (values, result) in [
            (&[true] as &[bool], true),
            (&[false], false),
            (&[true, false], false),
            (&[true, true, true], true),
        ] {
            let mut eval = Function {
                gate: Gate::And,
                inverted: false,
            }
            .evaluate();
            values.iter().for_each(|value| eval.add(*value));
            assert_eq!(eval.get(), result, "invalid result")
        }
    }

    #[test]
    pub fn evaluate_const() {
        for (values, c) in [
            (&[true] as &[bool], true),
            (&[false], false),
            (&[true, false], false),
            (&[true, true, true], true),
        ] {
            let mut eval = Function {
                gate: Gate::Constant(c),
                inverted: false,
            }
            .evaluate();
            values.iter().for_each(|value| eval.add(*value));
            assert_eq!(eval.get(), c, "invalid result")
        }
    }

    struct TryComputeTest {
        gate: Gate,
        target: bool,
        arity: Option<usize>,
        candidate_fn: &'static [(Option<bool>, Option<bool>, Option<bool>)],
        success: bool,
    }

    impl TryComputeTest {
        pub fn run(&self) {
            let mut max_i = -1;
            let result = self
                .gate
                .try_compute(self.target, self.arity, |i, req, pref| {
                    assert_eq!(max_i + 1, i as i32, "candidate_fn not called consecutive");
                    max_i = i as i32;
                    let values = self.candidate_fn[i];
                    assert_eq!(req, values.0, "invalid required for i = {i}");
                    assert_eq!(pref, values.1, "invalid preferred for i = {i}");
                    values.2.map(|v| (v, i))
                });
            if let Some(is) = result {
                assert!(self.success, "expected failure");
                match self.arity {
                    Some(arity) => {
                        for i in 0..arity {
                            assert_eq!(is[i], i);
                        }
                    }
                    None => {
                        for i in 0..is.len() {
                            assert_eq!(is[i], i)
                        }
                    }
                }
                assert_eq!(is.len(), self.candidate_fn.len());
            } else {
                assert!(!self.success, "expected success");
            }
            assert_eq!(
                max_i,
                self.candidate_fn.len() as i32 - 1,
                "not all candidate function calls ran"
            );
        }
    }

    #[test]
    fn try_compute_maj() {
        for test in [
            TryComputeTest {
                gate: Gate::Maj,
                target: true,
                arity: None,
                candidate_fn: &[(None, Some(true), Some(true))],
                success: true,
            },
            TryComputeTest {
                gate: Gate::Maj,
                target: true,
                arity: None,
                candidate_fn: &[
                    (None, Some(true), Some(false)),
                    (None, Some(true), Some(true)),
                    (None, Some(true), Some(true)),
                ],
                success: true,
            },
            TryComputeTest {
                gate: Gate::Maj,
                target: false,
                arity: None,
                candidate_fn: &[
                    (None, Some(false), Some(true)),
                    (None, Some(false), Some(false)),
                    (None, Some(false), Some(true)),
                    (None, Some(false), Some(false)),
                    (None, Some(false), Some(false)),
                ],
                success: true,
            },
            TryComputeTest {
                gate: Gate::Maj,
                target: true,
                arity: Some(1),
                candidate_fn: &[(Some(true), Some(true), Some(true))],
                success: true,
            },
            TryComputeTest {
                gate: Gate::Maj,
                target: true,
                arity: Some(3),
                candidate_fn: &[
                    (None, Some(true), Some(false)),
                    (Some(true), Some(true), Some(true)),
                    (Some(true), Some(true), Some(true)),
                ],
                success: true,
            },
            TryComputeTest {
                gate: Gate::Maj,
                target: true,
                arity: Some(3),
                candidate_fn: &[
                    (None, Some(true), Some(true)),
                    (None, Some(true), Some(false)),
                    (Some(true), Some(true), Some(true)),
                ],
                success: true,
            },
            TryComputeTest {
                gate: Gate::Maj,
                target: false,
                arity: Some(5),
                candidate_fn: &[
                    (None, Some(false), Some(true)),
                    (None, Some(false), Some(false)),
                    (None, Some(false), Some(true)),
                    (Some(false), Some(false), Some(false)),
                    (Some(false), Some(false), Some(false)),
                ],
                success: true,
            },
            TryComputeTest {
                gate: Gate::Maj,
                target: false,
                arity: Some(5),
                candidate_fn: &[
                    (None, Some(false), Some(false)),
                    (None, Some(false), Some(false)),
                    (None, Some(false), Some(false)),
                    (None, None, Some(false)),
                    (None, None, Some(false)),
                ],
                success: true,
            },
        ] {
            test.run();
        }
    }

    #[test]
    fn try_compute_and() {
        for test in [
            TryComputeTest {
                // true = and(true)
                gate: Gate::And,
                target: true,
                arity: None,
                candidate_fn: &[(Some(true), Some(true), Some(true))],
                success: true,
            },
            TryComputeTest {
                // true = and(true, true)
                gate: Gate::And,
                target: true,
                arity: Some(2),
                candidate_fn: &[
                    (Some(true), Some(true), Some(true)),
                    (Some(true), Some(true), Some(true)),
                ],
                success: true,
            },
            TryComputeTest {
                // true != and(false, *)
                gate: Gate::And,
                target: true,
                arity: Some(2),
                candidate_fn: &[(Some(true), Some(true), Some(false))],
                success: false,
            },
            TryComputeTest {
                // false = and(true, true, false)
                gate: Gate::And,
                target: false,
                arity: None,
                candidate_fn: &[
                    (None, Some(false), Some(true)),
                    (None, Some(false), Some(true)),
                    (None, Some(false), Some(false)),
                ],
                success: true,
            },
            TryComputeTest {
                // false = and(true, false)
                gate: Gate::And,
                target: false,
                arity: Some(2),
                candidate_fn: &[
                    (None, Some(false), Some(true)),
                    (Some(false), Some(false), Some(false)),
                ],
                success: true,
            },
            TryComputeTest {
                // false = and(false, *)
                gate: Gate::And,
                target: false,
                arity: Some(2),
                candidate_fn: &[(None, Some(false), Some(false)), (None, None, Some(true))],
                success: true,
            },
            TryComputeTest {
                // false = and(false, *)
                gate: Gate::And,
                target: false,
                arity: Some(2),
                candidate_fn: &[(None, Some(false), Some(false)), (None, None, None)],
                success: false,
            },
        ] {
            test.run();
        }
    }

    #[test]
    fn try_compute_constant() {
        for test in [
            // impossible
            TryComputeTest {
                gate: Gate::Constant(false),
                target: true,
                arity: Some(5),
                candidate_fn: &[],
                success: false,
            },
            TryComputeTest {
                gate: Gate::Constant(true),
                target: false,
                arity: None,
                candidate_fn: &[],
                success: false,
            },
            // possible
            TryComputeTest {
                gate: Gate::Constant(false),
                target: false,
                arity: Some(2),
                candidate_fn: &[(None, None, Some(true)), (None, None, Some(false))],
                success: true,
            },
            TryComputeTest {
                gate: Gate::Constant(true),
                target: true,
                arity: Some(2),
                candidate_fn: &[(None, None, Some(true)), (None, None, None)],
                success: false,
            },
        ] {
            test.run();
        }
    }
}
