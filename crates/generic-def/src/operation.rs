use std::{
    borrow::Cow,
    fmt::{self, Display},
    sync::Arc,
};

use derive_more::Deref;
use itertools::Itertools;

use crate::{CellType, Function, Operand, Operands};

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
    pub input_override: Option<InputIndices>,
    pub function: Function,
}

impl<CT> PartialEq for OperationType<CT> {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl<CT> Eq for OperationType<CT> {}

impl<CT> PartialOrd for OperationType<CT> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<CT> Ord for OperationType<CT> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.name.cmp(&other.name)
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Operation<CT> {
    pub typ: OperationType<CT>,
    pub inputs: Vec<Operand<CT>>,
    pub outputs: Vec<Operand<CT>>,
}

impl<CT> Display for Operation<CT>
where
    CT: CellType,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}({}({})",
            self.typ.name,
            self.typ.function,
            self.inputs.iter().format(", "),
        )?;
        if !self.outputs.is_empty() {
            write!(f, " -> {}", self.outputs.iter().format(", "))?;
        }
        write!(f, ")")?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InputIndices {
    All,
    Index(usize),
}
