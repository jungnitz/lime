use std::{fmt::Debug, hash::Hash, rc::Rc};

use eggmock::egg::{EGraph, Id, Language};
use lime_generic_def::Operation;

pub type ProgramEGraph<CT> = EGraph<ProgramLanguage<Operation<CT>>, ()>;

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum ProgramLanguage<O> {
    Seq([Id; 2]),
    Op(Rc<O>),
}

impl<O> Language for ProgramLanguage<O>
where
    O: Debug + Clone + Hash + Ord,
{
    type Discriminant = bool;

    fn discriminant(&self) -> Self::Discriminant {
        match self {
            Self::Seq(..) => true,
            Self::Op(..) => false,
        }
    }

    fn matches(&self, other: &Self) -> bool {
        self.discriminant() == other.discriminant()
    }

    fn children(&self) -> &[Id] {
        match self {
            Self::Seq(ids) => ids,
            Self::Op(..) => &[],
        }
    }

    fn children_mut(&mut self) -> &mut [Id] {
        match self {
            Self::Seq(ids) => ids,
            Self::Op(..) => &mut [],
        }
    }
}
