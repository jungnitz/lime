use std::borrow::Borrow;

use blanket::blanket;
use eggmock::Signal;
use lime_generic_def::{Cell, CellType};
use rustc_hash::FxHashMap;

/// Keeps track of the state of the cells in a memory array.
///
/// Contains mappings from
/// * cell to signal (1:1)
/// * and signal to cells (1:n)
#[blanket(derive(Mut))]
pub trait State<CT> {
    /// Returns the signal stored in the given cell if known.
    fn cell(&self, cell: Cell<CT>) -> Option<Signal>;
    /// Returns all cells that contain the given signal.
    fn cells_with(&self, signal: Signal) -> impl Iterator<Item = Cell<CT>> + '_;
    /// Sets the signal of the given cell.
    ///
    /// Returns the signal that the cell did store before this operation (which may be equal to the
    /// given signal if it did not change).
    fn set<Sig: Into<Option<Signal>>>(&mut self, cell: Cell<CT>, signal: Sig) -> Option<Signal>;
}

pub struct StateDiff<CT>(SimpleState<CT>);

impl<CT> StateDiff<CT> {
    pub fn apply_to(&self, state: &mut impl State<CT>)
    where
        CT: Copy,
    {
        for (cell, signal) in &self.0.cell_to_signal {
            state.set(*cell, *signal);
        }
    }
}

impl<CT> Default for StateDiff<CT> {
    fn default() -> Self {
        Self(Default::default())
    }
}

pub struct DiffState<'a, CT, S>(pub &'a mut StateDiff<CT>, pub &'a S);

impl<'a, CT, S> State<CT> for DiffState<'a, CT, S>
where
    CT: CellType,
    S: State<CT>,
{
    fn cell(&self, cell: Cell<CT>) -> Option<Signal> {
        self.0.0.cell(cell).or_else(|| self.1.borrow().cell(cell))
    }

    fn cells_with(&self, signal: Signal) -> impl Iterator<Item = Cell<CT>> + '_ {
        self.0.0.cells_with(signal).chain(
            self.1
                .borrow()
                .cells_with(signal)
                .filter(|cell| self.0.0.cell(*cell).is_none()),
        )
    }

    fn set<O: Into<Option<Signal>>>(&mut self, cell: Cell<CT>, signal: O) -> Option<Signal> {
        self.0
            .0
            .set(cell, signal)
            .or_else(|| self.1.borrow().cell(cell))
    }
}

#[derive(Debug, Clone)]
pub struct SimpleState<CT> {
    signal_to_cells: FxHashMap<Signal, Vec<Cell<CT>>>,
    cell_to_signal: FxHashMap<Cell<CT>, Option<Signal>>,
}

impl<CT> Default for SimpleState<CT> {
    fn default() -> Self {
        Self {
            signal_to_cells: Default::default(),
            cell_to_signal: Default::default(),
        }
    }
}

impl<CT: CellType> State<CT> for SimpleState<CT> {
    fn cell(&self, cell: Cell<CT>) -> Option<Signal> {
        self.cell_to_signal.get(&cell)?.as_ref().copied()
    }

    fn cells_with(&self, signal: Signal) -> impl Iterator<Item = Cell<CT>> + '_ {
        self.signal_to_cells
            .get(&signal)
            .into_iter()
            .flat_map(|cells| cells.iter().copied())
    }

    fn set<S: Into<Option<Signal>>>(&mut self, cell: Cell<CT>, signal: S) -> Option<Signal> {
        let signal = signal.into();
        let previous = self.cell_to_signal.insert(cell, signal).flatten();

        // add signal -> cell mapping
        if let Some(signal) = signal {
            self.signal_to_cells.entry(signal).or_default().push(cell);
        };

        // if a signal was already stored in this cell, we need to remove the reverse mapping
        // (signal -> cell) as well
        if let Some(previous) = previous {
            let cells = self.signal_to_cells.get_mut(&previous).unwrap();
            let idx = cells
                .iter()
                .position(|cell_for_previous| *cell_for_previous == cell)
                .unwrap();
            cells.swap_remove(idx);
        }

        previous
    }
}
