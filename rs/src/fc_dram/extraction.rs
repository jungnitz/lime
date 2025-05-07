//! Computation of Compiling Costs

use eggmock::egg::{CostFunction, Id};
use eggmock::{EggIdToSignal, MigLanguage, Mig, NetworkLanguage, Provider, Signal};
use either::Either;
use rustc_hash::FxHashMap;
use std::cell::RefCell;
use std::cmp::{max, Ordering};
use std::iter;
use std::ops::{Deref, Index};
use std::rc::Rc;
use super::architecture::FCDRAMArchitecture;
use super::compile;

pub struct CompilingCostFunction{}

// impl StackedPartialGraph { } // Do I need this??

/// TODO: add reliability as cost-metric
#[derive(Debug)]
pub struct CompilingCost {
    // partial: RefCell<Either<StackedPartialGraph, Rc<CollapsedPartialGraph>>>,
    program_cost: usize,
}

impl CostFunction<MigLanguage> for CompilingCostFunction {
    type Cost = Rc<CompilingCost>;

    /// Compute cost of given `enode`
    fn cost<C>(&mut self, enode: &MigLanguage, mut costs: C) -> Self::Cost
    where
        C: FnMut(Id) -> Self::Cost,
    {
        todo!()
        // let root = enode.clone();
        // let cost = match enode {
        //     MigLanguage::False | MigLanguage::Input(_) => CompilingCost::leaf(root),
        //     MigLanguage::Not(id) => {
        //         let cost = costs(*id);
        //
        //         let nesting = if cost.not_nesting == NotNesting::NotANot {
        //             NotNesting::FirstNot
        //         } else {
        //             NotNesting::NestedNots
        //         };
        //         CompilingCost::with_children(self.architecture, root, iter::once((*id, cost)), nesting)
        //     }
        //     MigLanguage::Maj(children) => CompilingCost::with_children(
        //         self.architecture,
        //         root,
        //         children.map(|id| (id, costs(id))),
        //         NotNesting::NotANot,
        //     ),
        // };
        // Rc::new(cost)
    }
}

impl CompilingCost {
    pub fn leaf(root: MigLanguage) -> Self {
        Self {
            // partial: RefCell::new(Either::Left(StackedPartialGraph::leaf(root))),
            program_cost: 0,
        }
    }

    pub fn with_children(
        root: MigLanguage,
        child_costs: impl IntoIterator<Item=(Id, Rc<CompilingCost>)>,
    ) -> Self {
        todo!()
        // let child_graphs = child_costs
        //     .into_iter()
        //     .map(|(id, cost)| cost.collapsed_graph(id));
        // let partial_graph = StackedPartialGraph::new(root, child_graphs);
        // let program_cost = compile(architecture, &partial_graph.with_backward_edges()).instructions.len();
        // Self {
        //     partial: RefCell::new(Either::Left(partial_graph)),
        //     not_nesting,
        //     program_cost,
        // }
    }

}

// impl StackedPartialGraph {
//     pub fn get_root_id(&self) -> Id {
//         Id::from(self.first_free_id + 1)
//     }
// }
//
// impl Index<Id> for StackedPartialGraph {
//     type Output = MigLanguage;
//
//     fn index(&self, index: Id) -> &Self::Output {
//         if index == self.get_root_id() {
//             &self.root
//         } else {
//             self.nodes.iter().filter_map(|m| m.get(&index)).next().unwrap()
//         }
//     }
// }
//
// impl Provider for StackedPartialGraph {
//     type Node = Mig;
//
//     fn outputs(&self) -> impl Iterator<Item=Signal> {
//         iter::once(self.to_signal(self.get_root_id()))
//     }
//
//     fn node(&self, id: eggmock::Id) -> Self::Node {
//         self[Id::from(id)]
//             .to_node(|id| self.to_signal(id))
//             .expect("id should point to a non-not node")
//     }
// }

impl PartialEq for CompilingCost {
    fn eq(&self, other: &Self) -> bool {
        todo!()
        // if other.not_nesting == NotNesting::NestedNots && self.not_nesting == NotNesting::NestedNots {
        //     true
        // } else {
        //     self.program_cost.eq(&other.program_cost)
        // }
    }
}

impl PartialOrd for CompilingCost {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        todo!()
        // #[allow(clippy::collapsible_else_if)]
        // if self.not_nesting == NotNesting::NestedNots {
        //     if other.not_nesting == NotNesting::NestedNots {
        //         Some(Ordering::Equal)
        //     } else {
        //         Some(Ordering::Greater)
        //     }
        // } else {
        //     if other.not_nesting == NotNesting::NestedNots {
        //         Some(Ordering::Less)
        //     } else {
        //         self.program_cost.partial_cmp(&other.program_cost)
        //     }
        // }
    }
}
