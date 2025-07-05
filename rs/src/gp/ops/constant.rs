use std::fmt::Display;

use itertools::Itertools;
use lime_generic_def::{
    Architecture, BoolHint, BoolSet, Cell, CellType, Function, FunctionEvaluation, InputIndices,
    Operand, OperandType, Operation, OperationType, Outputs,
};

use crate::gp::{
    Ambit, AmbitCellType, FELIX, FELIXCellType, IMPLY, IMPLYCellType, PLiM, PLiMCellType,
};

pub fn set<CT: CellType + Display>(
    arch: &Architecture<CT>,
    cell: Cell<CT>,
    value: bool,
) -> Vec<Operation<CT>> {
    let mut operations = Vec::new();
    for operation in arch.operations().iter() {
        set_using_operation(arch, &mut operations, operation, cell, value);
    }
    println!("{}", operations.iter().format("\n"));
    operations
}

fn set_using_operation<CT: CellType>(
    arch: &Architecture<CT>,
    operations: &mut Vec<Operation<CT>>,
    operation: &OperationType<CT>,
    cell: Cell<CT>,
    value: bool,
) {
    if operation.input_override.is_none() {
        set_using_output(arch, operations, operation, cell, value);
    }

    if arch.outputs().contains_none() {
        set_using_input_result(arch, operations, operation, cell, value);
    }
}

fn set_using_output<CT: CellType>(
    arch: &Architecture<CT>,
    operations: &mut Vec<Operation<CT>>,
    typ: &OperationType<CT>,
    cell: Cell<CT>,
    value: bool,
) {
    let inverted = match arch.outputs().fit_cell(cell) {
        BoolSet::None => return,
        BoolSet::All => None,
        BoolSet::Single(inverted) => Some(inverted),
    };
    let target = inverted.map(|inverted| value ^ inverted);
    'combinations: for combination in typ.input.combinations() {
        let mut map = ConstantMapping::new(typ.function, combination.len(), target);
        for operand in combination {
            if map.match_next(operand).is_none() {
                continue 'combinations;
            }
        }
        let result_value = map
            .eval
            .evaluate()
            .expect("should be able to evaluate function");
        let output = Operand {
            cell,
            inverted: result_value ^ value,
        };
        debug_assert!(
            inverted.is_none_or(|inverted| inverted == output.inverted),
            "output should fit specification"
        );
        operations.push(Operation {
            typ: typ.clone(),
            inputs: map.inputs,
            outputs: vec![output],
        });
    }
}

fn set_using_input_result<CT: CellType>(
    arch: &Architecture<CT>,
    operations: &mut Vec<Operation<CT>>,
    operation: &OperationType<CT>,
    cell: Cell<CT>,
    value: bool,
) {
    let Some(InputIndices::Index(target_idx)) = operation.input_override else {
        return;
    };
    'combinations: for combination in operation.input.combinations() {
        let typ = combination[target_idx];
        let inverted = match typ.fit(cell) {
            None => continue,
            Some(inverted) => inverted,
        };
        let mut target_func = operation.function;
        target_func.inverted ^= inverted;
        let mut mapping = ConstantMapping::new(target_func, combination.len(), Some(value));
        for (i, operand) in combination.iter().enumerate() {
            if i == target_idx {
                continue;
            }
            if mapping.match_next(operand).is_none() {
                continue 'combinations;
            }
        }
        match mapping.eval.hint(Some(combination.len()), value) {
            None | Some(BoolHint::Require(_)) => continue,
            _ => {}
        }
        mapping
            .inputs
            .insert(target_idx, Operand { cell, inverted });
        operations.push(Operation {
            typ: operation.clone(),
            inputs: mapping.inputs,
            outputs: vec![],
        });
    }
}

/// Keep track of used constants so that we do not use a constant cell twice
pub struct ConstantMapping<CT> {
    eval: FunctionEvaluation,
    inputs: Vec<Operand<CT>>,
    used: BoolSet,
    arity: usize,
    target: Option<bool>,
}

impl<CT: CellType> ConstantMapping<CT> {
    pub fn new(func: Function, arity: usize, target: Option<bool>) -> Self {
        Self {
            eval: func.evaluate(),
            inputs: Vec::with_capacity(arity),
            used: BoolSet::None,
            arity,
            target,
        }
    }
    #[must_use]
    pub fn match_next(&mut self, typ: &OperandType<CT>) -> Option<()> {
        // determine the requirements for the next value based on previously used cells
        let use_hint = match self.used {
            BoolSet::All => return None,
            BoolSet::Single(cell_value) => BoolHint::Require(!cell_value ^ typ.inverted),
            BoolSet::None => BoolHint::Any,
        };
        // determine what the function wants as the next argument
        let func_hint = match self.target {
            None => BoolHint::Any,
            Some(target) => self.eval.hint(Some(self.arity), target)?,
        };
        // combine requirements
        let hint = func_hint.and(use_hint)?;
        let (value, operand) = typ.try_fit_constant(hint)?;
        self.eval.add(value);
        self.used = self.used.insert(value ^ typ.inverted);
        self.inputs.push(operand);
        Some(())
    }
}
