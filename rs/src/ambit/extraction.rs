use super::{compile, Architecture};
use crate::opt_extractor::OptCostFunction;
use eggmock::egg::{Analysis, EClass, Id, Language};
use eggmock::{EggIdToSignal, Mig, MigLanguage, Network, NetworkLanguage, Signal};
use either::Either;
use rustc_hash::FxHashMap;
use std::cell::RefCell;
use std::cmp::{max, Ordering};
use std::iter;
use std::ops::{Deref, Index};
use std::rc::Rc;

pub struct CompilingCostFunction<'a> {
    pub architecture: &'a Architecture,
}

#[derive(Debug, Default, Eq, PartialEq)]
pub enum NotNesting {
    #[default]
    NotANot,
    FirstNot,
    NestedNots,
}

#[derive(Debug)]
pub struct StackedPartialGraph {
    nodes: Vec<Rc<FxHashMap<Id, MigLanguage>>>,
    first_free_id: usize,
    root: MigLanguage,
}

#[derive(Debug)]
pub struct CollapsedPartialGraph {
    nodes: Rc<FxHashMap<Id, MigLanguage>>,
    first_free_id: usize,
    root_id: Id,
}

impl StackedPartialGraph {
    pub fn leaf(node: MigLanguage) -> Self {
        Self {
            nodes: Vec::new(),
            first_free_id: 0,
            root: node,
        }
    }
    pub fn new(
        root: MigLanguage,
        child_graphs: impl IntoIterator<Item = Rc<CollapsedPartialGraph>>,
    ) -> Self {
        let mut nodes = Vec::new();
        let mut first_free_id = 0;
        for graph in child_graphs {
            nodes.push(graph.nodes.clone());
            first_free_id = max(first_free_id, graph.first_free_id);
        }
        Self {
            nodes,
            first_free_id,
            root,
        }
    }
    pub fn collapse(&self, real_id: Id) -> CollapsedPartialGraph {
        let mut nodes: FxHashMap<Id, MigLanguage> = FxHashMap::default();
        let first_free_id = max(self.first_free_id, usize::from(real_id));
        nodes.extend(
            self.nodes
                .iter()
                .flat_map(|map| map.iter().map(|(id, node)| (*id, node.clone()))),
        );
        nodes.insert(real_id, self.root.clone());
        CollapsedPartialGraph {
            nodes: Rc::new(nodes),
            first_free_id,
            root_id: real_id,
        }
    }
}

#[derive(Debug)]
pub struct CompilingCost {
    partial: RefCell<Either<StackedPartialGraph, Rc<CollapsedPartialGraph>>>,
    not_nesting: NotNesting,
    program_cost: usize,
}

impl<A: Analysis<MigLanguage>> OptCostFunction<MigLanguage, A> for CompilingCostFunction<'_> {
    type Cost = Rc<CompilingCost>;

    fn cost<C>(
        &mut self,
        eclass: &EClass<MigLanguage, A::Data>,
        enode: &MigLanguage,
        mut costs: C,
    ) -> Option<Self::Cost>
    where
        C: FnMut(Id) -> Self::Cost,
    {
        // detect self-cycles, other cycles will be detected by compiling, which will result in an
        // error
        if enode.children().contains(&eclass.id) {
            return None;
        }
        let root = enode.clone();
        let cost = match enode {
            MigLanguage::False | MigLanguage::Input(_) => CompilingCost::leaf(root),
            MigLanguage::Not(id) => {
                let cost = costs(*id);

                let nesting = if cost.not_nesting == NotNesting::NotANot {
                    NotNesting::FirstNot
                } else {
                    NotNesting::NestedNots
                };
                CompilingCost::with_children(
                    self.architecture,
                    root,
                    iter::once((*id, cost)),
                    nesting,
                )?
            }
            MigLanguage::Maj(children) => CompilingCost::with_children(
                self.architecture,
                root,
                children.map(|id| (id, costs(id))),
                NotNesting::NotANot,
            )?,
        };
        Some(Rc::new(cost))
    }
}

impl CompilingCost {
    pub fn leaf(root: MigLanguage) -> Self {
        Self {
            partial: RefCell::new(Either::Left(StackedPartialGraph::leaf(root))),
            not_nesting: NotNesting::NotANot,
            program_cost: 0,
        }
    }
    pub fn with_children(
        architecture: &Architecture,
        root: MigLanguage,
        child_costs: impl IntoIterator<Item = (Id, Rc<CompilingCost>)>,
        not_nesting: NotNesting,
    ) -> Option<Self> {
        let child_graphs = child_costs
            .into_iter()
            .map(|(id, cost)| cost.collapsed_graph(id));
        let partial_graph = StackedPartialGraph::new(root, child_graphs);
        let program_cost = match compile(architecture, &partial_graph.with_backward_edges()) {
            Err(_) => return None,
            Ok(program) => program.instructions.len(),
        };
        Self {
            partial: RefCell::new(Either::Left(partial_graph)),
            not_nesting,
            program_cost,
        }
        .into()
    }
    pub fn collapsed_graph(&self, id: Id) -> Rc<CollapsedPartialGraph> {
        let mut partial = self.partial.borrow_mut();
        let stacked = match partial.deref() {
            Either::Left(stacked) => stacked,
            Either::Right(collapsed) => {
                assert_eq!(collapsed.root_id, id);
                return collapsed.clone();
            }
        };
        let collapsed = Rc::new(stacked.collapse(id));
        *partial = Either::Right(collapsed.clone());
        collapsed
    }
}

impl StackedPartialGraph {
    pub fn get_root_id(&self) -> Id {
        Id::from(self.first_free_id + 1)
    }
}

impl Index<Id> for StackedPartialGraph {
    type Output = MigLanguage;

    fn index(&self, index: Id) -> &Self::Output {
        if index == self.get_root_id() {
            &self.root
        } else {
            self.nodes
                .iter()
                .filter_map(|m| m.get(&index))
                .next()
                .unwrap()
        }
    }
}

impl Network for StackedPartialGraph {
    type Node = Mig;

    fn outputs(&self) -> impl Iterator<Item = Signal> {
        iter::once(self.to_signal(self.get_root_id()))
    }

    fn node(&self, id: eggmock::Id) -> Self::Node {
        self[Id::from(id)]
            .to_node(|id| self.to_signal(id))
            .expect("id should point to a non-not node")
    }
}

impl PartialEq for CompilingCost {
    fn eq(&self, other: &Self) -> bool {
        if other.not_nesting == NotNesting::NestedNots && self.not_nesting == NotNesting::NestedNots
        {
            true
        } else {
            self.program_cost.eq(&other.program_cost)
        }
    }
}

impl PartialOrd for CompilingCost {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        #[allow(clippy::collapsible_else_if)]
        if self.not_nesting == NotNesting::NestedNots {
            if other.not_nesting == NotNesting::NestedNots {
                Some(Ordering::Equal)
            } else {
                Some(Ordering::Greater)
            }
        } else {
            if other.not_nesting == NotNesting::NestedNots {
                Some(Ordering::Less)
            } else {
                self.program_cost.partial_cmp(&other.program_cost)
            }
        }
    }
}
