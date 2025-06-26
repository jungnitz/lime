use std::fmt::{self, Display};

use derive_where::derive_where;

use super::{Architecture, Function, Operand, Operands};

#[derive_where(Debug, Clone)]
pub struct OperationType<A: Architecture> {
    pub input: Operands<A>,
    pub input_results: &'static [InputResult],
    pub output: Option<Function>,
}

#[derive_where(Debug)]
pub struct Operation<A: Architecture> {
    pub typ: OperationType<A>,
    pub inputs: Vec<Operand<A>>,
    pub outputs: Vec<Operand<A>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InputResult {
    Unchanged,
    Destroyed,
    Function(Function),
}
