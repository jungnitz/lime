use std::{
    cmp::Ordering,
    collections::hash_map::Entry,
    fmt::{Debug, Display},
    iter::{self, Chain},
    ops::Index,
};

use derive_more::Index;
use either::Either;
use itertools::Itertools;
use lime_generic_def::{Architecture, Cell, CellIndex, CellType, Operand, OperandType, Operation};
use petgraph::prelude::DiGraphMap;
use rustc_hash::FxHashMap;

use crate::gp::{
    CellOrVar, OperationCost,
    copy::{discovery::find_copy_operations, discovery_constant::find_set_constant},
};

pub const FROM_VAR: CellIndex = 0;
pub const TO_VAR: CellIndex = 1;

#[derive(Debug)]
pub struct Edge<CT, Cost> {
    pub inverted: bool,
    pub template: Vec<Operation<CellOrVar<CT>, CT>>,
    pub cost: Cost,
}

#[derive(Default)]
struct TypeNode<V> {
    value: V,
    children: FxHashMap<CellIndex, V>,
}

impl<V> TypeNode<V> {
    pub fn value(&self, idx: Option<CellIndex>) -> Option<&V> {
        match idx {
            None => Some(&self.value),
            Some(idx) => self.children.get(&idx),
        }
    }
    pub fn value_or_default(&mut self, idx: Option<CellIndex>) -> &mut V
    where
        V: Default,
    {
        match idx {
            None => &mut self.value,
            Some(idx) => self.children.entry(idx).or_default(),
        }
    }
}

struct TypeNodes<CT, V>(FxHashMap<CT, TypeNode<V>>);

impl<CT: CellType, V> TypeNodes<CT, V> {
    fn get(&self, node: Node<CT>) -> Option<&V> {
        match node {
            Node::Cell(cell) => self.0.get(&cell.typ())?.children.get(&cell.index()),
            Node::Type(typ) => Some(&self.0.get(&typ)?.value),
        }
    }
    fn iter(&self) -> impl Iterator<Item = (Node<CT>, &V)> {
        self.0.iter().flat_map(|(typ, node)| {
            std::iter::once((Node::Type(*typ), &node.value)).chain(
                node.children
                    .iter()
                    .map(|(idx, value)| (Node::Cell(Cell::new(*typ, *idx)), value)),
            )
        })
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Node<CT> {
    Type(CT),
    Cell(Cell<CT>),
}

type FromTypeNode<CT, Cost> = TypeNodes<CT, [Option<Edge<CT, Cost>>; 2]>;

pub struct CopyGraph<CT, Cost> {
    nodes: TypeNodes<CT, FromTypeNode<CT, Cost>>,
}

pub struct FindParams<'a, CT, OC: OperationCost<CT>> {
    pub arch: &'a Architecture<CT>,
    pub cost: &'a OC,
    pub graph: &'a mut CopyGraph<CT, OC::Cost>,
}

impl<CT: CellType, Cost> CopyGraph<CT, Cost> {
    pub fn build(arch: &Architecture<CT>, cost: &impl OperationCost<CT, Cost = Cost>) -> Self {
        let mut graph = Self {
            nodes: Default::default(),
        };
        let mut params = FindParams {
            arch,
            cost,
            graph: &mut graph,
        };
        find_set_constant(&mut params);
        find_copy_operations(&mut params);
        graph
    }

    pub fn all_optimal_edges_matching(
        &self,
        from: Node<CT>,
        to: Node<CT>,
        inverted: bool,
    ) -> impl Iterator<Item = (Node<CT>, Node<CT>, &Edge<CT, Cost>)> + '_
    where
        Cost: Ord,
    {
        // determine all relevant values for the typeset based on the "matching node"
        // the value of Node::Type(_) is the last element, which is important
        fn relevant_nodes<CT: CellType, V>(
            typenodes: &TypeNodes<CT, V>,
            node: Node<CT>,
        ) -> impl Iterator<Item = (Node<CT>, &V)> + '_ {
            typenodes
                .0
                .get(&node.typ())
                .into_iter()
                .flat_map(move |typenode| {
                    match node {
                        Node::Type(typ) => Either::Left(
                            typenode
                                .children
                                .iter()
                                .map(move |(idx, edges)| (Cell::new(typ, *idx), edges)),
                        ),
                        Node::Cell(cell) => {
                            let edge = typenode.children.get(&cell.index());
                            Either::Right(edge.map(|edge| (cell, edge)).into_iter())
                        }
                    }
                    .map(|(cell, edges)| (Node::Cell(cell), edges))
                    .chain(iter::once((Node::Type(node.typ()), &typenode.value)))
                })
        }

        let mut all_covered = false;
        relevant_nodes(&self.nodes, from)
            .map_while(move |(from_node, edges)| {
                if all_covered {
                    // we already have a full cover, we are done
                    return None;
                }
                let mut all_to_covered = false;
                let edges = relevant_nodes(edges, to)
                    .map_while(move |(to_node, edges)| {
                        if all_to_covered {
                            return None;
                        }
                        Some(edges[inverted as usize].as_ref().map(|edge| {
                            all_to_covered =
                                matches!(to_node, Node::Type(_)) || matches!(to, Node::Cell(_));
                            (from_node, to_node, edge)
                        }))
                    })
                    .flatten();
                all_covered = all_to_covered
                    && (matches!(from_node, Node::Type(_)) || matches!(from, Node::Cell(_)));
                Some(edges)
            })
            .flatten()
    }

    pub fn consider_edge(&mut self, from: Node<CT>, to: Node<CT>, edge: Edge<CT, Cost>)
    where
        Cost: PartialOrd + Clone,
    {
        let inverted = edge.inverted as usize;
        let from_typenode = self.nodes.0.entry(from.typ()).or_default();
        if let Node::Cell(cell) = from {
            if let Some(to_typenode) = from_typenode.value.0.get(&to.typ()) {
                // if from's parent already points to the target's parent with a smaller cost we do
                // not need this
                if let Some(existing_edge) = &to_typenode.value[inverted]
                    && edge.cost >= existing_edge.cost
                {
                    return;
                }
                // if from's parent already points to the target with a smaller cost we do not need
                // this
                if let Node::Cell(to_cell) = to
                    && let Some(existing_edge) = to_typenode
                        .children
                        .get(&to_cell.index())
                        .map(|edges| edges[inverted].as_ref())
                        .flatten()
                    && edge.cost >= existing_edge.cost
                {
                    return;
                }
            }
        }
        let from_edges = from_typenode.value_or_default(from.index());
        let to_typenode = from_edges.0.entry(to.typ()).or_default();

        // if from already points to the target's parent with a smaller cost we do not need this
        if let Node::Cell(to_cell) = to
            && let Some(existing_edge) = &to_typenode.value[inverted]
            && edge.cost >= existing_edge.cost
        {
            return;
        }

        // insert the new edge
        let edge_cost = edge.cost.clone();
        let current = &mut to_typenode.value_or_default(to.index())[inverted];
        match current {
            None => *current = Some(edge),
            Some(current) if current.cost > edge_cost => *current = edge,
            _ => {
                // better solution was in place, no-op
                return;
            }
        };

        // now we might have added an edge that is a better solution than others, let's see...

        // We will use this closure later to decide which edge entries to retain. It removes the
        // edge if it is more expensive than the newly added edge and returns true if the array
        // still contains something afterward.
        let check_retain = move |edges: &mut [Option<Edge<CT, Cost>>; 2]| -> bool {
            // does it have an edge and if yes, is it cheaper? if not, remove it
            let opt_edge = &mut edges[inverted];
            if let Some(edge) = opt_edge
                && edge.cost >= edge_cost
            {
                *opt_edge = None;
            }
            // remove from map if candidate has no more associated edges
            edges.iter().any(|opt| opt.is_some())
        };

        // if we have an edge from the from node, but to a child of the to node with equal or worse
        // cost, we can remove that edge:
        if let Node::Type(_) = to {
            to_typenode.children.retain(|_, edges| check_retain(edges));
        }

        // if we have an edge from any of from's child nodes to to or any of its children with equal
        // or more cost, we can remove that edge as well:
        if let Node::Type(_) = from {
            from_typenode.children.retain(|_, from_edges| {
                let Entry::Occupied(mut to_typenode_entry) = from_edges.0.entry(to.typ()) else {
                    return true;
                };
                let to_typenode = to_typenode_entry.get_mut();
                match to.index() {
                    // to is a type node, we may delete edges to the type value and children
                    None => {
                        to_typenode.children.retain(|_, edges| check_retain(edges));
                        check_retain(&mut to_typenode.value);
                    }
                    // to is a cell node, hence we may only delete edges to the respective child
                    Some(idx) => {
                        let Entry::Occupied(mut entry) = to_typenode.children.entry(idx) else {
                            return true;
                        };
                        if !check_retain(entry.get_mut()) {
                            entry.remove();
                        }
                    }
                };
                // did we delete all edges to the type of to? if yes, remove the entry
                if to_typenode.value.iter().all(|opt| opt.is_none())
                    && to_typenode.children.is_empty()
                {
                    to_typenode_entry.remove();
                }
                // did we remove the last entry for this cell of the from-type?
                !from_edges.0.is_empty()
            });
        }
    }
}

impl<CT: CellType, Cost> Edge<CT, Cost> {
    pub fn instantiate<TargetCT>(
        &self,
        from: Cell<TargetCT>,
        to: Cell<TargetCT>,
    ) -> impl Iterator<Item = Operation<TargetCT, CT>>
    where
        TargetCT: CellType,
        CT: Into<TargetCT>,
    {
        let map_operand = move |operand: &Operand<CellOrVar<CT>>| -> Operand<TargetCT> {
            let idx = operand.cell.index();
            let cell = match operand.cell.typ() {
                CellOrVar::Var if idx == FROM_VAR => from,
                CellOrVar::Var if idx == TO_VAR => to,
                CellOrVar::Var => panic!("invalid variable index"),
                CellOrVar::Cell(typ) => Cell::new(typ.into(), idx),
            };
            Operand {
                cell,
                inverted: operand.inverted,
            }
        };
        self.template.iter().map(move |operation| Operation {
            typ: operation.typ.clone(),
            inputs: Vec::from_iter(operation.inputs.iter().map(map_operand)),
            outputs: Vec::from_iter(operation.outputs.iter().map(map_operand)),
        })
    }
}

impl<CT: CellType> Node<CT> {
    pub fn intersect(self, other: Self) -> Option<Self> {
        match self {
            Self::Type(typ) => {
                if typ == other.typ() {
                    Some(other)
                } else {
                    None
                }
            }
            Self::Cell(self_cell) => match other {
                Self::Cell(other_cell) => {
                    if self_cell == other_cell {
                        Some(self)
                    } else {
                        None
                    }
                }
                Self::Type(_) => other.intersect(self), // use case from above
            },
        }
    }
    pub fn for_type(op: OperandType<CT>) -> Self {
        match op.index {
            Some(index) => Self::Cell(Cell::new(op.typ, index)),
            None => Self::Type(op.typ),
        }
    }
    pub fn for_cell(cell: Cell<CT>, use_index: bool) -> Self {
        if use_index {
            Self::Cell(cell)
        } else {
            Self::Type(cell.typ())
        }
    }
    pub fn contains(self, other: Node<CT>) -> bool {
        match (self, other) {
            (Self::Type(typ), other) => other.typ() == typ,
            (Self::Cell(c1), Self::Cell(c2)) => c1 == c2,
            (Self::Cell(_), Self::Type(_)) => false,
        }
    }
    pub fn typ(self) -> CT {
        match self {
            Self::Type(typ) => typ,
            Self::Cell(cell) => cell.typ(),
        }
    }
    pub fn index(self) -> Option<CellIndex> {
        match self {
            Self::Type(_) => None,
            Self::Cell(cell) => Some(cell.index()),
        }
    }
}

impl<CT: CellType, Cost: Debug> Debug for CopyGraph<CT, Cost> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "CopyGraph (\n");
        for (from, to_edges) in self.nodes.iter() {
            for (to, edges) in to_edges.iter() {
                for (inverted, edge) in [true, false].into_iter().filter_map(|inverted| {
                    edges[inverted as usize]
                        .as_ref()
                        .map(|edge| (inverted, edge))
                }) {
                    write!(f, "  {from} -> ")?;
                    if edge.inverted {
                        write!(f, "!")?;
                    }
                    write!(f, "{to} with cost {:?}\n", edge.cost)?;
                    for operation in &edge.template {
                        write!(f, "    {operation}\n")?;
                    }
                }
            }
        }
        write!(f, ")");
        Ok(())
    }
}

impl<CT: CellType> Display for Node<CT> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cell(cell) => write!(f, "{cell}"),
            Self::Type(typ) => write!(f, "{}", typ.name()),
        }
    }
}

impl<CT, V: Default> Default for TypeNodes<CT, V> {
    fn default() -> Self {
        Self(Default::default())
    }
}
