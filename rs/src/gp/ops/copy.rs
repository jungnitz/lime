use lime_generic_def::{
    Architecture, BoolHint, BoolSet, Cell, CellType, InputIndices, Operand, Operation,
    OperationType,
};

use crate::gp::{
    ops::constant::{ConstantMapping, set},
    program::{ProgramVersion, ProgramVersions},
};

pub fn copy<CT: CellType>(
    versions: &mut impl ProgramVersions<CT>,
    from_cell: Cell<CT>,
    to: Operand<CT>,
) {
    let arch = versions.arch().clone();
    for op in arch.operations().iter() {
        copy_using_output(&arch, op, versions, from_cell, to);
        copy_using_input_override(&arch, op, versions, from_cell, to.cell, to.inverted);
    }
}

fn copy_using_input_override<CT: CellType>(
    arch: &Architecture<CT>,
    op: &OperationType<CT>,
    versions: &mut impl ProgramVersions<CT>,
    from_cell: Cell<CT>,
    to_cell: Cell<CT>,
    inverted: bool,
) {
    if !arch.outputs().contains_none() {
        return;
    }
    let Some(InputIndices::Index(to_idx)) = op.input_override else {
        return;
    };
    for combination in op.input.combinations() {
        let to_operand = &combination[to_idx];
        let Some(to_inverted) = to_operand.fit(to_cell) else {
            continue;
        };
        'from: for (from_idx, from) in combination.iter().enumerate() {
            let from = &combination[from_idx];
            let Some(from_inverted) = from.fit(from_cell) else {
                continue;
            };
            let ident_inverted = to_inverted ^ from_inverted ^ inverted;
            let mut mapping =
                ConstantMapping::new_to_ident(op.function, combination.len(), Some(ident_inverted));
            for (i, operand) in combination.iter().enumerate() {
                if i == from_idx || i == to_idx {
                    continue;
                }
                if mapping.match_next(operand).is_none() {
                    continue 'from;
                }
            }
            let to_hint = mapping
                .eval
                .hint_id(Some(combination.len()), Some(ident_inverted));
            let Some(to_hint) = to_hint else {
                continue 'from;
            };
            let mut version = versions.new();
            if let BoolHint::Require(to_value) = to_hint {
                set(&mut version.branch(), to_cell, to_value ^ to_inverted);
                mapping.eval.add(to_value);
            } else {
                mapping.eval.add_unknown();
            }
            if mapping.eval.id_inverted() != Some(ident_inverted) {
                continue 'from;
            }
            mapping.inputs.insert(
                from_idx,
                Operand {
                    cell: from_cell,
                    inverted: from_inverted,
                },
            );
            mapping.inputs.insert(
                to_idx,
                Operand {
                    cell: to_cell,
                    inverted: to_inverted,
                },
            );
            version.append(Operation {
                typ: op.clone(),
                inputs: mapping.inputs,
                outputs: vec![],
            });
            version.save();
        }
    }
}

fn copy_using_output<CT: CellType>(
    arch: &Architecture<CT>,
    op: &OperationType<CT>,
    versions: &mut impl ProgramVersions<CT>,
    from_cell: Cell<CT>,
    to: Operand<CT>,
) {
    if !op.input_override.is_none() {
        return;
    }
    let output_inverted = match arch.outputs().fit_cell(to.cell) {
        BoolSet::None => return,
        BoolSet::Single(value) => Some(value),
        BoolSet::All => None,
    };
    for combination in op.input.combinations() {
        'from: for (from_idx, from) in combination.iter().enumerate() {
            let from_inverted = match from.fit(from_cell) {
                None => continue 'from,
                Some(inverted) => inverted,
            };
            // we want:
            //   to.cell = to.inverted ^ from_cell
            // we know that if we find a (possibly inverted) identity:
            //   to.cell = (from_inverted ^ output_inverted ^ identity_inverted) from_cell
            // hence: from_inverted ^ output_inverted ^ identity_inverted = to.inverted
            //    <=> from_inverted ^ output_inverted ^ to.inverted = identity_inverted
            // if output can be both - inverted and not inverted - we can freely choose whether the
            // identity has to be inverting
            let ident_inverted = output_inverted
                .map(|output_inverted| output_inverted ^ from_inverted ^ to.inverted);
            let mut mapping =
                ConstantMapping::new_to_ident(op.function, combination.len(), ident_inverted);
            for (i, operand) in combination.iter().enumerate() {
                if i == from_idx {
                    continue;
                }
                if mapping.match_next(operand).is_none() {
                    continue 'from;
                }
            }
            let ident_inverted = match mapping.eval.id_inverted() {
                None => continue 'from,
                Some(inverted) => {
                    if let Some(ident_inverted) = ident_inverted
                        && ident_inverted != inverted
                    {
                        continue 'from;
                    } else {
                        inverted
                    }
                }
            };
            let output_inverted = from_inverted ^ to.inverted ^ ident_inverted;
            mapping.inputs.insert(
                from_idx,
                Operand {
                    cell: from_cell,
                    inverted: from_inverted,
                },
            );
            let mut version = versions.new();
            version.append(Operation {
                typ: op.clone(),
                inputs: mapping.inputs,
                outputs: vec![Operand {
                    cell: to.cell,
                    inverted: output_inverted,
                }],
            });
            version.save();
        }
    }
}
