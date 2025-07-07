mod egraph;

use std::fmt::Display;

use itertools::Itertools;
use lime_generic_def::{Architecture, CellType, Operation};

use crate::gp::state::{DiffState, SimpleState, State, StateDiff};

pub trait ProgramVersion<CT> {
    fn append(&mut self, op: Operation<CT>);
    fn state(&mut self) -> impl State<CT>;
    fn save(self);
    fn branch(&mut self) -> impl ProgramVersions<CT>;
}

pub trait ProgramVersions<CT> {
    fn arch(&self) -> &Architecture<CT>;
    fn new(&mut self) -> impl ProgramVersion<CT>;
}

pub struct Program<CT> {
    arch: Architecture<CT>,
    instructions: Vec<Operation<CT>>,
    state: SimpleState<CT>,
}

impl<CT: CellType> Program<CT> {
    pub fn new(arch: Architecture<CT>) -> Self {
        Self {
            arch,
            instructions: Default::default(),
            state: Default::default(),
        }
    }
    pub fn append_with_min_cost(&mut self) -> impl ProgramVersions<CT> {
        MinCostAppender {
            arch: &self.arch,
            base_state: &mut self.state,
            target: &mut self.instructions,
            min: None,
        }
    }
    pub fn arch(&self) -> &Architecture<CT> {
        &self.arch
    }
}

impl<CT> Display for Program<CT>
where
    Operation<CT>: Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.instructions.iter().format("\n").fmt(f)
    }
}

struct MinCostAppender<'p, CT: CellType, S: State<CT>> {
    arch: &'p Architecture<CT>,
    base_state: S,
    target: &'p mut Vec<Operation<CT>>,
    min: Option<(Vec<Operation<CT>>, StateDiff<CT>)>,
}

struct MinCostAppenderVersion<'a, 'p, CT: CellType, S: State<CT>> {
    appender: &'a mut MinCostAppender<'p, CT, S>,
    instructions: Vec<Operation<CT>>,
    diff: StateDiff<CT>,
}

impl<'a, 'p, CT, S> ProgramVersion<CT> for MinCostAppenderVersion<'a, 'p, CT, S>
where
    CT: CellType,
    S: State<CT>,
{
    fn append(&mut self, op: Operation<CT>) {
        self.instructions.push(op);
    }

    fn state(&mut self) -> impl State<CT> {
        DiffState(&mut self.diff, &self.appender.base_state)
    }

    fn branch(&mut self) -> impl ProgramVersions<CT> {
        MinCostAppender {
            arch: &self.appender.arch,
            base_state: DiffState(&mut self.diff, &self.appender.base_state),
            target: &mut self.instructions,
            min: None,
        }
    }
    fn save(self) {
        let self_new = match &self.appender.min {
            None => true,
            Some((instructions, diff)) => instructions.len() < self.instructions.len(),
        };
        if self_new {
            self.appender.min = Some((self.instructions, self.diff))
        }
    }
}

impl<'p, CT, S> ProgramVersions<CT> for MinCostAppender<'p, CT, S>
where
    CT: CellType,
    S: State<CT>,
{
    fn new(&mut self) -> impl ProgramVersion<CT> {
        MinCostAppenderVersion {
            appender: self,
            diff: Default::default(),
            instructions: Vec::new(),
        }
    }

    fn arch(&self) -> &Architecture<CT> {
        self.arch
    }
}

impl<'p, CT: CellType, S: State<CT>> Drop for MinCostAppender<'p, CT, S> {
    fn drop(&mut self) {
        if let Some((instructions, diff)) = self.min.take() {
            self.target.extend(instructions);
            diff.apply_to(&mut self.base_state);
        }
    }
}
