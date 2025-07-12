use std::iter;

use either::Either;
use itertools::Itertools;
use lime_generic_def::{BoolHint, Cell, CellType, InputIndices, Operand, Operation, OperationType};
use tracing::{debug, info, warn};

use crate::gp::{
    CellOrVar, OperationCost,
    copy::constant_mapping::ConstantMapping,
    copy::graph::{Edge, FROM_VAR, FindParams, Node, TO_VAR},
};

pub fn find_copy_operations<CT: CellType, CF: OperationCost<CT>>(
    params: &mut FindParams<'_, CT, CF>,
) {
    let arch = params.arch.clone();
    for operation in arch.operations().iter() {
        for inverted in [true, false] {
            find_using_input_override(params, operation, inverted);
            find_using_output(params, operation, inverted);
        }
    }
}

fn find_using_input_override<CT: CellType, CF: OperationCost<CT>>(
    params: &mut FindParams<CT, CF>,
    typ: &OperationType<CT>,
    inverted: bool,
) {
    if inverted {
        return; //TODO: REMOVE
    }
    // determine to which operand the result will be written
    let Some(InputIndices::Index(to_idx)) = typ.input_override else {
        return;
    };
    for combination in typ.input.combinations() {
        let to = combination[to_idx];
        for (from_idx, from) in combination.iter().enumerate() {
            if from_idx == to_idx {
                continue;
            }
            let mut mapping =
                ConstantMapping::new_to_ident(typ.function, combination.len(), Some(inverted));
            // try to map all operands other than to and from
            let Ok(mut inputs) = combination
                .iter()
                .enumerate()
                .filter(|(i, _)| *i != to_idx && *i != from_idx)
                .map(|(_, op)| mapping.try_match_varop(op))
                .collect::<Result<Vec<_>, _>>()
            else {
                continue;
            };
            let Some(to_hint) = mapping
                .eval
                .hint_id(Some(combination.len()), Some(inverted))
            else {
                continue;
            };
            // now we have two scenarios:
            // 1. the function is already known to be a identity no matter what value to currently
            //    holds in which case we are effectively done here
            // 2. the function may become an identity if to has the correct value for it, which we
            //    try to enforce using a previously discovered copy operation (i.e. setting it to a
            //    constant value)
            let mut templates = Vec::new();
            if let BoolHint::Require(to_value) = to_hint {
                mapping.eval.add(to_value);

                let const_cell = CT::constant(to_value);
                for (from_node, to_node, edge) in params.graph.all_optimal_edges_matching(
                    Node::Cell(const_cell),
                    Node::for_type(to),
                    to.inverted,
                ) {
                    templates.push(
                        edge.instantiate(
                            const_cell.map_type(CellOrVar::from),
                            Cell::new(CellOrVar::Var, TO_VAR),
                        )
                        .collect_vec(),
                    )
                }
            } else {
                mapping.eval.add_unknown();
                templates.push(vec![]);
            };
            if mapping.eval.id_inverted() != Some(inverted) {
                warn!(
                    ?typ,
                    ?mapping,
                    ?inputs,
                    "mapping did not result in the expected identity"
                );
                continue;
            }
            inputs.insert(
                from_idx,
                Operand {
                    cell: Cell::new(CellOrVar::<CT>::Var, FROM_VAR),
                    inverted: from.inverted,
                },
            );
            inputs.insert(
                to_idx,
                Operand {
                    cell: Cell::new(CellOrVar::<CT>::Var, TO_VAR),
                    inverted: to.inverted,
                },
            );
            for mut template in templates {
                let operation = Operation {
                    typ: typ.clone(),
                    inputs: inputs.clone(),
                    outputs: vec![],
                };
                let cost = params.cost.cost(&operation);
                let cost = template
                    .iter()
                    .fold(cost, |cost, op| cost + params.cost.cost(&op));
                template.push(operation);
                params.graph.consider_edge(
                    Node::for_type(*from),
                    Node::for_type(to),
                    Edge {
                        inverted: to.inverted ^ from.inverted ^ inverted,
                        cost,
                        template,
                    },
                );
            }
        }
        let to_operand = combination[to_idx];
    }
}

fn find_using_output<CT: CellType, CF: OperationCost<CT>>(
    params: &mut FindParams<CT, CF>,
    typ: &OperationType<CT>,
    inverted: bool,
) {
    if !typ.input_override.is_none() {
        return;
    }
    for combination in typ.input.combinations() {
        for (from_idx, from) in combination.iter().enumerate() {
            let mut mapping =
                ConstantMapping::new_to_ident(typ.function, combination.len(), Some(inverted));
            let Ok(mut inputs) = combination
                .iter()
                .enumerate()
                .filter(|(i, _)| *i != from_idx)
                .map(|(_, op)| mapping.try_match_varop(op))
                .collect::<Result<Vec<_>, _>>()
            else {
                continue;
            };
            if mapping.eval.id_inverted() != Some(inverted) {
                if combination.len() != 1 {
                    warn!(
                        ?typ,
                        ?mapping,
                        ?inputs,
                        "mapping did not result in the expected identity"
                    );
                }
                continue;
            }
            inputs.insert(
                from_idx,
                Operand {
                    cell: Cell::new(CellOrVar::Var, FROM_VAR),
                    inverted: from.inverted,
                },
            );
            for output in params.arch.outputs().single_operands() {
                let operation = Operation {
                    typ: typ.clone(),
                    inputs: inputs.clone(),
                    outputs: vec![Operand {
                        cell: Cell::new(CellOrVar::Var, TO_VAR),
                        inverted: output.inverted,
                    }],
                };
                let cost = params.cost.cost(&operation);
                params.graph.consider_edge(
                    Node::for_type(*from),
                    Node::for_type(output),
                    Edge {
                        inverted: inverted ^ output.inverted ^ from.inverted,
                        template: vec![operation],
                        cost,
                    },
                );
            }
        }
    }
}
