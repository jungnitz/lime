use lime_generic_def::{
    BoolHint, BoolSet, CellType, Function, FunctionEvaluation, Operand, OperandType,
};

use crate::gp::CellOrVar;

/// Keep track of used constants so that we do not use a constant cell twice
#[derive(derive_more::Debug)]
pub struct ConstantMapping<H> {
    pub eval: FunctionEvaluation,
    used: BoolSet,
    #[debug(skip)]
    hint: H,
}

impl<H> ConstantMapping<H> {
    pub fn new(func: Function, hint: H) -> Self {
        Self {
            eval: func.evaluate(),
            used: BoolSet::None,
            hint,
        }
    }
}

impl ConstantMapping<()> {
    pub fn new_with_target(
        func: Function,
        arity: usize,
        target: Option<bool>,
    ) -> ConstantMapping<impl FnMut(&mut FunctionEvaluation) -> Option<BoolHint>> {
        ConstantMapping::new(func, move |eval: &mut FunctionEvaluation| match target {
            None => Some(BoolHint::Any),
            Some(value) => eval.hint(Some(arity), value),
        })
    }
    pub fn new_to_ident(
        func: Function,
        arity: usize,
        inverted: Option<bool>,
    ) -> ConstantMapping<impl FnMut(&mut FunctionEvaluation) -> Option<BoolHint>> {
        ConstantMapping::new(func, move |eval: &mut FunctionEvaluation| {
            eval.hint_id(Some(arity), inverted)
        })
    }
}

impl<H> ConstantMapping<H>
where
    H: FnMut(&mut FunctionEvaluation) -> Option<BoolHint>,
{
    #[must_use]
    pub fn match_next<CT: CellType>(&mut self, typ: &OperandType<CT>) -> Option<Operand<CT>> {
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
        Some(operand)
    }

    pub fn try_match_varop<CT: CellType>(
        &mut self,
        op: &OperandType<CT>,
    ) -> Result<Operand<CellOrVar<CT>>, ()> {
        self.match_next(op)
            .map(|operand| operand.map_cell_type(CellOrVar::Cell))
            .ok_or(())
    }
}
