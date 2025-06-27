use std::{borrow::Cow, sync::Arc};

use derive_more::Deref;

use super::{Function, Operand, Operands};

#[derive(Debug, Clone, Deref)]
#[deref(forward)]
pub struct Operations<CT>(Arc<[OperationType<CT>]>);

impl<CT> Operations<CT> {
    pub fn new(operations: Vec<OperationType<CT>>) -> Self {
        Self(operations.into())
    }
}

#[derive(Debug, Clone)]
pub struct OperationType<CT> {
    pub name: Cow<'static, str>,
    pub input: Operands<CT>,
    pub input_results: InputResults,
    pub output: Option<Function>,
}

#[derive(Debug)]
pub struct Operation<CT> {
    pub typ: OperationType<CT>,
    pub inputs: Vec<Operand<CT>>,
    pub outputs: Vec<Operand<CT>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InputResult {
    Unchanged,
    Function(Function),
}

#[derive(Debug, Clone, Deref)]
#[deref(forward)]
pub struct InputResults(Arc<[InputResult]>);

impl InputResults {
    pub fn new(results: Vec<InputResult>) -> Self {
        if results
            .iter()
            .all(|result| *result == InputResult::Unchanged)
        {
            return Self(vec![].into());
        }
        Self(results.into())
    }
    pub fn all(result: InputResult) -> Self {
        Self::new(vec![result])
    }
    pub fn new_unchanged() -> Self {
        Self::new(Vec::new())
    }

    pub fn arity(&self) -> Option<usize> {
        if self.0.len() <= 1 {
            None
        } else {
            Some(self.0.len())
        }
    }
    pub fn unchanged(&self) -> bool {
        self.0.is_empty()
    }
}
