use std::{fmt::Debug, hash::Hash};

use eggmock::egg::{EGraph, Id, Language};

use super::Operation;

pub type ProgramEGraph<A> = EGraph<ProgramLanguage<Operation<A>>, ()>;

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum ProgramLanguage<O> {
    Seq([Id; 2]),
    Op(O),
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
