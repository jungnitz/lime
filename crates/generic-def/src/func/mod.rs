mod and;
mod constant;
mod maj;

use std::fmt::Display;

use delegate::delegate;
use strum::EnumString;

use crate::{
    BoolHint, display_maybe_inverted,
    func::{and::AndEval, constant::ConstEval, maj::MajEval},
};

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
    pub fn evaluate(&self) -> GateEvaluation {
        match self {
            Self::And => GateEvaluation::And(AndEval::new()),
            Self::Maj => GateEvaluation::Maj(MajEval::new()),
            Self::Constant(c) => GateEvaluation::Const(ConstEval::new(*c)),
        }
    }

    /// Try to find a set of input values so that the value of this gate is the given target
    /// value.
    ///
    /// The candidate function is called once for each input value index or, if no arity is given,
    /// until the target value is reached and has the following arguments:
    ///
    /// - The current input argument index (in `0..arity` or `0..`) and for the input at that index
    /// - a hint that indicates valid or "good" choices for each input value
    ///
    /// The return value of the candidate function indicates the chosen value for this input as the
    /// first value. The second value is collected into the returned `Vec` (in order of the input
    /// argument indices). When returning `None` from the candidate function at any time, this means
    /// that no candidate was found and therefore this function returns `None` as well.
    pub fn try_compute<R>(
        self,
        target: bool,
        arity: Option<usize>,
        mut candidate_fn: impl FnMut(usize, BoolHint) -> Option<(bool, R)>,
    ) -> Option<Vec<R>> {
        let mut results = match arity {
            Some(arity) => Vec::with_capacity(arity),
            None => Vec::new(),
        };
        let mut candidate_fn = |i, hint| {
            let (value, r) = candidate_fn(i, hint)?;
            match hint {
                BoolHint::Require(required) if required != value => None,
                _ => Some((value, r)),
            }
        };

        let mut eval = self.evaluate();
        for i in 0.. {
            match arity {
                Some(arity) if i == arity => break,
                None if eval.evaluate() == Some(target) => break,
                _ => {}
            }
            let hint = eval.hint(arity, target)?;
            let (value, r) = candidate_fn(i, hint)?;
            results.push(r);
            eval.add(value);
        }
        debug_assert_eq!(
            eval.evaluate(),
            Some(target),
            "did not compute target value correctly"
        );
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
        candidate_fn: impl FnMut(usize, BoolHint) -> Option<(bool, R)>,
    ) -> Option<Vec<R>> {
        self.gate
            .try_compute(target ^ self.inverted, arity, candidate_fn)
    }
    pub fn evaluate(self) -> FunctionEvaluation {
        FunctionEvaluation {
            inverted: self.inverted,
            gate: self.gate.evaluate(),
        }
    }
}

impl Display for Function {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        display_maybe_inverted(f, self.inverted)?;
        write!(f, "{}", self.gate)
    }
}

trait EvaluationMethods {
    fn hint(&self, arity: Option<usize>, target: bool) -> Option<BoolHint>;
    fn hint_to_ident(&self, arity: Option<usize>, inverted: bool) -> Option<BoolHint>;
    fn add(&mut self, value: bool);
    fn evaluate(&self) -> Option<bool>;
}

pub struct FunctionEvaluation {
    inverted: bool,
    gate: GateEvaluation,
}

impl FunctionEvaluation {
    pub fn hint(&self, arity: Option<usize>, target: bool) -> Option<BoolHint> {
        self.gate.hint(arity, target ^ self.inverted)
    }
    pub fn hint_to_ident(&self, arity: Option<usize>, inverted: bool) -> Option<BoolHint> {
        self.gate.hint_to_ident(arity, inverted ^ self.inverted)
    }
    pub fn add(&mut self, value: bool) {
        self.gate.add(value);
    }
    pub fn evaluate(&self) -> Option<bool> {
        self.gate.evaluate().map(|v| v ^ self.inverted)
    }
}

pub enum GateEvaluation {
    And(AndEval),
    Maj(MajEval),
    Const(ConstEval),
}

impl GateEvaluation {
    delegate! {
        to match self {
            Self::And(and) => and,
            Self::Maj(maj) => maj,
            Self::Const(c) => c,
        } {
            fn hint(&self, arity: Option<usize>, target: bool) -> Option<BoolHint>;
            fn hint_to_ident(&self, arity: Option<usize>, inverted: bool) -> Option<BoolHint>;
            fn add(&mut self, value: bool);
            fn evaluate(&self) -> Option<bool>;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::BoolHint::*;
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
            assert_eq!(eval.evaluate(), Some(result), "invalid result")
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
            assert_eq!(eval.evaluate(), Some(result), "invalid result")
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
            assert_eq!(eval.evaluate(), Some(c), "invalid result")
        }
    }

    #[derive(Debug)]
    struct TryComputeTest {
        gate: Gate,
        target: bool,
        arity: Option<usize>,
        candidate_fn: &'static [(BoolHint, Option<bool>)],
        success: bool,
    }

    impl TryComputeTest {
        pub fn run(&self) {
            let mut max_i = -1;
            let result = self.gate.try_compute(self.target, self.arity, |i, hint| {
                assert_eq!(max_i + 1, i as i32, "candidate_fn not called consecutive");
                max_i = i as i32;
                let values = self.candidate_fn[i];
                assert_eq!(hint, values.0, "invalid hint for i = {i}");
                values.1.map(|v| (v, i))
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
                candidate_fn: &[(Prefer(true), Some(true))],
                success: true,
            },
            TryComputeTest {
                gate: Gate::Maj,
                target: true,
                arity: None,
                candidate_fn: &[
                    (Prefer(true), Some(false)),
                    (Prefer(true), Some(true)),
                    (Prefer(true), Some(true)),
                ],
                success: true,
            },
            TryComputeTest {
                gate: Gate::Maj,
                target: false,
                arity: None,
                candidate_fn: &[
                    (Prefer(false), Some(true)),
                    (Prefer(false), Some(false)),
                    (Prefer(false), Some(true)),
                    (Prefer(false), Some(false)),
                    (Prefer(false), Some(false)),
                ],
                success: true,
            },
            TryComputeTest {
                gate: Gate::Maj,
                target: true,
                arity: Some(1),
                candidate_fn: &[(Require(true), Some(true))],
                success: true,
            },
            TryComputeTest {
                gate: Gate::Maj,
                target: true,
                arity: Some(3),
                candidate_fn: &[
                    (Prefer(true), Some(false)),
                    (Require(true), Some(true)),
                    (Require(true), Some(true)),
                ],
                success: true,
            },
            TryComputeTest {
                gate: Gate::Maj,
                target: true,
                arity: Some(3),
                candidate_fn: &[
                    (Prefer(true), Some(true)),
                    (Prefer(true), Some(false)),
                    (Require(true), Some(true)),
                ],
                success: true,
            },
            TryComputeTest {
                gate: Gate::Maj,
                target: false,
                arity: Some(5),
                candidate_fn: &[
                    (Prefer(false), Some(true)),
                    (Prefer(false), Some(false)),
                    (Prefer(false), Some(true)),
                    (Require(false), Some(false)),
                    (Require(false), Some(false)),
                ],
                success: true,
            },
            TryComputeTest {
                gate: Gate::Maj,
                target: false,
                arity: Some(5),
                candidate_fn: &[
                    (Prefer(false), Some(false)),
                    (Prefer(false), Some(false)),
                    (Prefer(false), Some(false)),
                    (Any, Some(false)),
                    (Any, Some(false)),
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
                candidate_fn: &[(Require(true), Some(true))],
                success: true,
            },
            TryComputeTest {
                // true = and(true, true)
                gate: Gate::And,
                target: true,
                arity: Some(2),
                candidate_fn: &[(Require(true), Some(true)), (Require(true), Some(true))],
                success: true,
            },
            TryComputeTest {
                // true != and(false, *)
                gate: Gate::And,
                target: true,
                arity: Some(2),
                candidate_fn: &[(Require(true), Some(false))],
                success: false,
            },
            TryComputeTest {
                // false = and(true, true, false)
                gate: Gate::And,
                target: false,
                arity: None,
                candidate_fn: &[
                    (Prefer(false), Some(true)),
                    (Prefer(false), Some(true)),
                    (Prefer(false), Some(false)),
                ],
                success: true,
            },
            TryComputeTest {
                // false = and(true, false)
                gate: Gate::And,
                target: false,
                arity: Some(2),
                candidate_fn: &[(Prefer(false), Some(true)), (Require(false), Some(false))],
                success: true,
            },
            TryComputeTest {
                // false = and(false, *)
                gate: Gate::And,
                target: false,
                arity: Some(2),
                candidate_fn: &[(Prefer(false), Some(false)), (Any, Some(true))],
                success: true,
            },
            TryComputeTest {
                // false = and(false, *)
                gate: Gate::And,
                target: false,
                arity: Some(2),
                candidate_fn: &[(Prefer(false), Some(false)), (Any, None)],
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
                candidate_fn: &[(Any, Some(true)), (Any, Some(false))],
                success: true,
            },
            TryComputeTest {
                gate: Gate::Constant(true),
                target: true,
                arity: Some(2),
                candidate_fn: &[(Any, Some(true)), (Any, None)],
                success: false,
            },
        ] {
            test.run();
        }
    }
}
