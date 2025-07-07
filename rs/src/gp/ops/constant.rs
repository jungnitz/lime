use lime_generic_def::{
    Architecture, BoolHint, BoolSet, Cell, CellType, Function, FunctionEvaluation, InputIndices,
    Operand, OperandType, Operation, OperationType,
};

use crate::gp::program::{ProgramVersion, ProgramVersions};

pub fn set<CT: CellType>(versions: &mut impl ProgramVersions<CT>, cell: Cell<CT>, value: bool) {
    let arch = versions.arch().clone();
    for operation in arch.operations().iter() {
        if operation.input_override.is_none() {
            set_using_output(&arch, versions, operation, cell, value);
        }

        if arch.outputs().contains_none() {
            set_using_input_result(&arch, versions, operation, cell, value);
        }
    }
}

fn set_using_output<CT: CellType>(
    arch: &Architecture<CT>,
    versions: &mut impl ProgramVersions<CT>,
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
        let mut map = ConstantMapping::new_with_target(typ.function, combination.len(), target);
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
        let mut version = versions.new();
        version.append(Operation {
            typ: typ.clone(),
            inputs: map.inputs,
            outputs: vec![output],
        });
        version.save();
    }
}

fn set_using_input_result<CT: CellType>(
    arch: &Architecture<CT>,
    versions: &mut impl ProgramVersions<CT>,
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
        let mut mapping =
            ConstantMapping::new_with_target(target_func, combination.len(), Some(value));
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
        let mut version = versions.new();
        version.append(Operation {
            typ: operation.clone(),
            inputs: mapping.inputs,
            outputs: vec![],
        });
        version.save();
    }
}

/// Keep track of used constants so that we do not use a constant cell twice
pub struct ConstantMapping<CT, H> {
    pub eval: FunctionEvaluation,
    pub inputs: Vec<Operand<CT>>,
    used: BoolSet,
    hint: H,
}

impl<CT, H> ConstantMapping<CT, H> {
    pub fn new(func: Function, hint: H) -> Self {
        Self {
            eval: func.evaluate(),
            inputs: Vec::new(),
            used: BoolSet::None,
            hint,
        }
    }
}

impl<CT> ConstantMapping<CT, ()> {
    pub fn new_with_target(
        func: Function,
        arity: usize,
        target: Option<bool>,
    ) -> ConstantMapping<CT, impl FnMut(&mut FunctionEvaluation) -> Option<BoolHint>> {
        ConstantMapping::new(func, move |eval: &mut FunctionEvaluation| match target {
            None => Some(BoolHint::Any),
            Some(value) => eval.hint(Some(arity), value),
        })
    }
    pub fn new_to_ident(
        func: Function,
        arity: usize,
        inverted: Option<bool>,
    ) -> ConstantMapping<CT, impl FnMut(&mut FunctionEvaluation) -> Option<BoolHint>> {
        ConstantMapping::new(func, move |eval: &mut FunctionEvaluation| {
            eval.hint_id(Some(arity), inverted)
        })
    }
}

impl<CT, H> ConstantMapping<CT, H>
where
    CT: CellType,
    H: FnMut(&mut FunctionEvaluation) -> Option<BoolHint>,
{
    #[must_use]
    pub fn match_next(&mut self, typ: &OperandType<CT>) -> Option<()> {
        // determine the requirements for the next value based on previously used cells
        let use_hint = match self.used {
            BoolSet::All => return None,
            BoolSet::Single(cell_value) => BoolHint::Require(!cell_value ^ typ.inverted),
            BoolSet::None => BoolHint::Any,
        };
        // combine requirements
        let hint = (self.hint)(&mut self.eval)?.and(use_hint)?;
        let (value, operand) = typ.try_fit_constant(hint)?;
        self.eval.add(value);
        self.used = self.used.insert(value ^ typ.inverted);
        self.inputs.push(operand);
        Some(())
    }
}
