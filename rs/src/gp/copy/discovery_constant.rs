use itertools::Itertools;
use lime_generic_def::{
    Architecture, BoolHint, Cell, CellType, FunctionEvaluation, InputIndices, Operand, OperandType,
    Operation, OperationType,
};
use tracing::{debug, warn};

use crate::gp::{
    CellOrVar, OperationCost,
    copy::constant_mapping::ConstantMapping,
    copy::graph::{Edge, FindParams, Node, TO_VAR},
};

use super::graph::CopyGraph;

/// Finds all side effect free copy operations from a constant cell that can be found by using the
/// value of the cell.
pub fn find_set_constant<CT: CellType, CF: OperationCost<CT>>(params: &mut FindParams<'_, CT, CF>) {
    let arch = params.arch.clone();
    for operation in arch.operations().iter() {
        for value in [true, false] {
            find_for_output(params, &operation, value);
            find_for_input_result(params, &operation, value);
        }
    }
}

fn find_for_input_result<CT: CellType, CF: OperationCost<CT>>(
    params: &mut FindParams<'_, CT, CF>,
    typ: &OperationType<CT>,
    value: bool,
) {
    if !params.arch.outputs().contains_none() {
        return;
    }
    let Some(InputIndices::Index(target_idx)) = typ.input_override else {
        return;
    };
    for combination in typ.input.combinations() {
        let mut mapping =
            ConstantMapping::new_with_target(typ.function, combination.len(), Some(value));
        let Ok(mut inputs) = combination
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != target_idx)
            .map(|(_, operand)| mapping.try_match_varop(operand))
            .collect::<Result<Vec<_>, ()>>()
        else {
            continue;
        };
        let hint = mapping.eval.hint(Some(combination.len()), value);
        match mapping.eval.hint(Some(combination.len()), value) {
            None | Some(BoolHint::Require(_)) => continue,
            _ => {}
        }
        mapping.eval.add_unknown();
        let result_value = mapping.eval.evaluate();
        if result_value != Some(value) {
            warn!(
                ?typ,
                ?combination,
                ?value,
                ?result_value,
                ?mapping,
                "did not get expected value for constant mapping"
            );
            continue;
        }
        let to = combination[target_idx];
        inputs.push(Operand {
            cell: Cell::new(CellOrVar::Var, TO_VAR),
            inverted: to.inverted,
        });
        add_edges(
            params,
            Operation {
                typ: typ.clone(),
                inputs: inputs,
                outputs: vec![],
            },
            value,
            to,
        );
    }
}

fn find_for_output<CT: CellType, CF: OperationCost<CT>>(
    params: &mut FindParams<'_, CT, CF>,
    typ: &OperationType<CT>,
    value: bool,
) {
    if !typ.input_override.is_none() {
        return;
    }
    for combination in typ.input.combinations() {
        let mut mapping =
            ConstantMapping::new_with_target(typ.function, combination.len(), Some(value));
        let Ok(inputs) = combination
            .iter()
            .map(|operand| mapping.try_match_varop(operand))
            .collect::<Result<Vec<_>, _>>()
        else {
            continue;
        };
        let result_value = mapping.eval.evaluate();
        let value = match result_value {
            Some(result_value) if value == result_value => result_value,
            _ => {
                warn!(
                    ?typ,
                    ?combination,
                    ?value,
                    ?result_value,
                    ?mapping,
                    "did not get expected value for constant mapping"
                );
                match result_value {
                    None => return,
                    Some(value) => value,
                }
            }
        };
        for to in params.arch.outputs().single_operands() {
            add_edges(
                params,
                Operation {
                    typ: typ.clone(),
                    inputs: inputs.clone(),
                    outputs: vec![Operand {
                        cell: Cell::new(CellOrVar::Var, TO_VAR),
                        inverted: to.inverted,
                    }],
                },
                value,
                to,
            );
        }
    }
}

/// Assuming that an operation with the given type and inputs produces the given value and that
/// value is written into to, add the relevant edges to this the graph.
fn add_edges<CT: CellType, CF: OperationCost<CT>>(
    params: &mut FindParams<'_, CT, CF>,
    operation: Operation<CellOrVar<CT>, CT>,
    value: bool,
    to: OperandType<CT>,
) {
    let cost = params.cost.cost(&operation);
    for from_value in [true, false] {
        let graph = &mut params.graph;
        let from_node = Node::Cell(CT::constant(from_value));
        let to_node = Node::for_type(to);
        let to_cell = Cell::new(CellOrVar::<CT>::Var, TO_VAR);

        let edge = Edge {
            inverted: value ^ from_value ^ to.inverted,
            template: vec![operation.clone()],
            cost: cost.clone(),
        };
        graph.consider_edge(from_node, to_node, edge);
    }
}
