mod dsl;
mod ops;
mod program;
mod state;

use std::fmt::Debug;
use std::hash::Hash;

pub type CellIndex = u32;

/// Abstractly describes a Logic-in-Memory architecture.
pub trait Architecture: Sized + 'static {
    type CellType: CellType;
    type Cell: Cell;

    fn outputs(&self) -> &'static [OperandSet<Self>];
    fn operations(&self) -> &'static [OperationType<Self>];

    fn cell(&self, typ: Self::CellType, idx: CellIndex) -> Self::Cell;
}

pub trait CellType: Copy + Debug + Eq + Hash {
    type Arch: Architecture;

    /// Number of cells of this type or `None` if infinite amount is available.
    fn count(self) -> Option<CellIndex>;
}

pub trait Cell: Copy + Debug + Eq + Hash {
    type Arch: Architecture;

    fn typ(self) -> <Self::Arch as Architecture>::CellType;
    fn index(self) -> CellIndex;
}

pub enum OperandType<A: Architecture> {
    Constant,
    Cell {
        typ: A::CellType,
        inverted: bool,
        index: Option<CellIndex>,
    },
}

pub enum Operand<A: Architecture> {
    Constant(bool),
    Cell(A::Cell),
}

pub enum OperandSet<A: Architecture> {
    Cross(&'static [OperandType<A>]),
    Set(&'static [&'static [OperandType<A>]]),
}

pub struct OperationType<A: Architecture> {
    pub input: OperandSet<A>,
    pub input_results: Vec<InputFunction>,
    pub output: Function,
}

pub struct Operation<A: Architecture> {
    pub typ: OperationType<A>,
    pub operands: Vec<Operand<A>>,
    pub outputs: Vec<A::Cell>,
}

pub struct Function {
    pub inverted: bool,
    pub gate: Gate,
}

pub enum Gate {
    And(Vec<bool>),
    Maj(Vec<bool>),
}

pub enum InputFunction {
    Unchanged,
    Destroyed,
    Function(Function),
}
