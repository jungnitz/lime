use std::{
    borrow::Cow,
    fmt::{self, Display},
    ops::Index,
    sync::Arc,
};

use derive_more::Deref;
use itertools::Itertools;

use crate::{CellType, Function, Operand, Operands, display_maybe_inverted};

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

impl<CT> Display for Operation<CT>
where
    CT: Display + CellType,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}({}", self.typ.name, self.inputs.iter().format(", "),)?;
        if !self.outputs.is_empty() {
            write!(f, " -> {}) //", self.outputs.iter().format(", "))?;
            let func = self
                .typ
                .output
                .expect("operand type should have an output if output operands are present");
            for output in self.outputs.iter() {
                display_operand_result(f, output, &func)?;
            }
        } else {
            write!(f, ") //")?;
        }
        for (i, input) in self.inputs.iter().enumerate() {
            let result = self.typ.input_results[i];
            match result {
                InputResult::Unchanged => continue,
                InputResult::Function(func) => display_operand_result(f, input, &func)?,
            }
        }
        Ok(())
    }
}

fn display_operand_result<CT: Display + CellType>(
    f: &mut fmt::Formatter<'_>,
    op: &Operand<CT>,
    func: &Function,
) -> fmt::Result {
    let cell = op.cell;
    let gate = func.gate;
    let invert = func.inverted ^ op.inverted;
    write!(f, " {cell} = ")?;
    display_maybe_inverted(f, invert)?;
    write!(f, "{gate};")
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

impl Index<usize> for InputResults {
    type Output = InputResult;

    fn index(&self, index: usize) -> &Self::Output {
        if self.is_empty() {
            &InputResult::Unchanged
        } else if index >= self.len() {
            &self.0[self.0.len() - 1]
        } else {
            &self.0[index]
        }
    }
}
