use std::{collections::hash_map::Entry, hash::Hash};

use eggmock::Signal;
use lime_generic_def::{Cell, CellType};
use rustc_hash::FxHashMap;

use super::Architecture;

/// Keeps track of the state of the cells in a memory array.
///
/// Contains mappings from
/// * cell to signal (1:1)
/// * and signal to cells (1:n)
#[derive(Default)]
pub struct State<CT> {
    signal_to_cells: FxHashMap<Signal, Vec<Cell<CT>>>,
    cell_to_signal: FxHashMap<Cell<CT>, Signal>,
}

impl<CT: CellType> State<CT> {
    /// Returns the signal stored in the given cell if known.
    pub fn cell(&self, cell: Cell<CT>) -> Option<Signal> {
        self.cell_to_signal.get(&cell).copied()
    }

    /// Returns all cells that contain the given signal.
    pub fn cells_with(&self, signal: Signal) -> impl Iterator<Item = Cell<CT>> + '_ {
        self.signal_to_cells
            .get(&signal)
            .into_iter()
            .flat_map(|cells| cells.iter().copied())
    }

    /// Sets the signal of the given cell.
    ///
    /// Returns the signal that the cell did store before this operation (which may be equal to the
    /// given signal if it did not change).
    pub fn set<S: Into<Option<Signal>>>(&mut self, cell: Cell<CT>, signal: S) -> Option<Signal> {
        let signal = signal.into();
        let entry = self.cell_to_signal.entry(cell);
        let previous = match entry {
            Entry::Occupied(mut entry) => {
                let previous = *entry.get();
                if Some(previous) == signal {
                    // signal already set correctly, therefore we are done already
                    return Some(previous);
                }
                match signal {
                    Some(signal) => entry.insert(signal),
                    None => entry.remove(),
                };
                Some(previous)
            }
            Entry::Vacant(entry) => {
                if let Some(signal) = signal {
                    entry.insert(signal);
                } else {
                    return None;
                }
                None
            }
        };

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
