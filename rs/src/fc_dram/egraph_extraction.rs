//! Computation of Compiling Costs

use eggmock::egg::{CostFunction, Id};
use eggmock::{EggIdToSignal, AigLanguage, Aig, NetworkLanguage, Network, Signal};
use std::cmp::Ordering;
use std::rc::Rc;

pub struct CompilingCostFunction{}

// impl StackedPartialGraph { } // Do I need this??

/// TODO: add reliability as cost-metric
#[derive(Debug)]
pub struct CompilingCost {
    // partial: RefCell<Either<StackedPartialGraph, Rc<CollapsedPartialGraph>>>,
    /// Probability that the whole program will run successfully
    success_rate: f64,
    /// Estimation of program cost (from input logic-ops)
    program_cost: usize,
}

impl CostFunction<AigLanguage> for CompilingCostFunction {
    type Cost = Rc<CompilingCost>;

    /// Compute cost of given `enode` using `cost_fn`
    ///
    /// Parameters determining cost of an enode:
    /// - distance of row-operands to sense amplifiers
    /// - operation:
    ///     - AND=<TODO>
    ///     - OR=<TODO>
    ///     - NOT=<TODO>
    ///
    /// TODO: NEXT
    fn cost<C>(&mut self, enode: &AigLanguage, mut cost_fn: C) -> Self::Cost
    where
        C: FnMut(Id) -> Self::Cost,
    {
        let cost = match enode {
            AigLanguage::False => 1,
            AigLanguage::Input(_node) => {
                // FCDRAMArchitecture::get_distance_of_row_to_sense_amps(&self, row)
                2
            },
            AigLanguage::And([_node1, _node2]) => {
                3
            },
            AigLanguage::Not(_node) => {
                4
            },
        };

        Rc::new(CompilingCost {
            success_rate: 0.0,
            program_cost: cost,
        })
        // todo!()

        // let root = enode.clone();
        // let cost = match enode {
        //     AigLanguage::False | AigLanguage::Input(_) => CompilingCost::leaf(root),
        //     AigLanguage::Not(id) => {
        //         let cost = costs(*id);
        //
        //         let nesting = if cost.not_nesting == NotNesting::NotANot {
        //             NotNesting::FirstNot
        //         } else {
        //             NotNesting::NestedNots
        //         };
        //         CompilingCost::with_children(self.architecture, root, iter::once((*id, cost)), nesting)
        //     }
        //     AigLanguage::Maj(children) => CompilingCost::with_children(
        //         self.architecture,
        //         root,
        //         children.map(|id| (id, costs(id))),
        //         NotNesting::NotANot,
        //     ),
        // };
        // Rc::new(cost)
    }
}

impl PartialEq for CompilingCost {
    fn eq(&self, other: &Self) -> bool {
        self.success_rate == other.success_rate && self.program_cost == other.program_cost
    }
}

/// First compare based on success-rate, then on program-cost
/// TODO: more fine-grained comparison !!
impl PartialOrd for CompilingCost {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.success_rate == other.success_rate {
            self.program_cost.partial_cmp(&other.program_cost)
        } else {
            self.success_rate.partial_cmp(&other.success_rate)
        }
    }
}
