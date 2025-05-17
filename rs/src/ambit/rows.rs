use super::{Architecture, BitwiseOperand, SingleRowAddress};
use eggmock::{Id, Mig, NetworkWithBackwardEdges, Signal};
use rustc_hash::FxHashMap;
use std::collections::hash_map::Entry;

/// Equivalent to a DRAM row.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum Row {
    In(u64),
    Out(u64),
    Spill(u32),
    Const(bool),
    Bitwise(BitwiseRow),
}

/// Equivalent to either a DCC row or a T row in the bitwise row section. DCC row always refers to
/// the non-inverted row.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum BitwiseRow {
    DCC(u8),
    T(u8),
}

/// Contains a snapshot state of the rows in an Ambit-like DRAM
#[derive(Debug, Clone)]
pub struct Rows<'a> {
    signals: FxHashMap<Signal, Vec<Row>>,
    rows: FxHashMap<Row, Signal>,
    spill_counter: u32,
    architecture: &'a Architecture,
}

impl<'a> Rows<'a> {
    /// Initializes the rows with the leaf values in the given network.
    pub fn new(
        ntk: &impl NetworkWithBackwardEdges<Node = Mig>,
        architecture: &'a Architecture,
    ) -> Self {
        let mut rows = Rows {
            signals: FxHashMap::default(),
            rows: FxHashMap::default(),
            spill_counter: 0,
            architecture,
        };
        rows.add_leafs(ntk);
        rows
    }

    fn add_leafs(&mut self, ntk: &impl NetworkWithBackwardEdges<Node = Mig>) {
        let leafs = ntk.leaves();
        self.rows.reserve(leafs.size_hint().0);
        for id in leafs {
            let node = ntk.node(id);
            match node {
                Mig::Input(i) => {
                    self.set_empty_row_signal(Row::In(i), Signal::new(id, false));
                }
                Mig::False => {
                    let signal = Signal::new(id, false);
                    self.set_empty_row_signal(Row::Const(false), signal);
                    self.set_empty_row_signal(Row::Const(true), signal.invert());
                }
                _ => unreachable!("leaf node should be either an input or a constant"),
            };
        }
    }

    pub fn get_free_dcc(&self) -> Option<u8> {
        for i in 0..self.architecture.num_dcc {
            if self.rows.get(&Row::Bitwise(BitwiseRow::DCC(i))).is_none() {
                return Some(i);
            }
        }
        None
    }

    /// Returns the current signal of the given row.
    pub fn get_row_signal(&self, row: Row) -> Option<Signal> {
        self.rows.get(&row).cloned()
    }

    /// Returns the signal of the given address. That is, if it is a DCC address the signal of the
    /// respective DCC row, but inverted if the address refers to the inverted row, otherwise the
    /// signal of the row of the address.
    pub fn get_address_signal(&self, address: SingleRowAddress) -> Option<Signal> {
        self.get_row_signal(address.row())?
            .maybe_invert(address.inverted())
            .into()
    }

    /// Returns the signal of the given operand. That is, if it is a DCC operand the signal of the
    /// respective DCC row, but inverted if the address refers to the inverted row, otherwise the
    /// signal of the row of the operand.
    pub fn get_operand_signal(&self, operand: BitwiseOperand) -> Option<Signal> {
        self.get_address_signal(operand.into())
    }

    /// Returns all rows with the given signal.
    pub fn get_rows(&self, signal: Signal) -> impl Iterator<Item = Row> + '_ {
        self.signals.get(&signal).into_iter().flatten().cloned()
    }

    /// Returns true iff a signal with the given id is stored in a row.
    pub fn contains_id(&self, id: Id) -> bool {
        self.get_rows(Signal::new(id, false)).next().is_some()
            || self.get_rows(Signal::new(id, true)).next().is_some()
    }

    /// Adds a new spill row with the given signal and returns its id.
    pub fn add_spill(&mut self, signal: Signal) -> u32 {
        self.spill_counter += 1;
        self.set_empty_row_signal(Row::Spill(self.spill_counter), signal);
        self.spill_counter
    }

    /// Sets the current signal of the given operand. That is, if it is a T-operand, sets the signal
    /// of the T-row to the given signal or, if it is a DCC-operand, sets the DCC-row to the signal,
    /// but inverted iff the operand is inverted.
    ///
    /// Returns the signal of the operand previous to this operation if it was changed.
    pub fn set_signal(&mut self, address: SingleRowAddress, signal: Signal) -> Option<Signal> {
        self.set_row_signal(address.row(), signal.maybe_invert(address.inverted()))?
            .maybe_invert(address.inverted())
            .into()
    }

    /// Equivalent to `set_row_signals`, but additionally ensures that the row was previously empty
    /// or contained the same signal.
    fn set_empty_row_signal(&mut self, row: Row, signal: Signal) {
        assert_eq!(
            self.set_row_signal(row, signal),
            None,
            "row {row:?} should be empty"
        )
    }

    /// Sets the signal of the given row, updating `self.rows` and `self.signals` accordingly.
    /// Returns the previous signal of the given row if it was changed.
    fn set_row_signal(&mut self, row: Row, signal: Signal) -> Option<Signal> {
        let row_entry = self.rows.entry(row);

        // detach previous signal
        let prev = match &row_entry {
            Entry::Occupied(v) => {
                let prev = *v.get();
                if prev == signal {
                    // signal already correctly set
                    return None;
                }
                Some(prev)
            }
            _ => None,
        };
        if let Some(prev) = prev {
            let prev_locations = self.signals.get_mut(&prev).unwrap();
            prev_locations.swap_remove(prev_locations.iter().position(|r| *r == row).unwrap());
        }

        // set new signal
        row_entry.insert_entry(signal);
        self.signals.entry(signal).or_default().push(row);

        prev
    }

    pub fn free_id_rows(&mut self, id: Id) {
        let non_inv = Signal::new(id, false);
        let inv = Signal::new(id, true);
        for sig in [non_inv, inv] {
            let Some(rows) = self.signals.remove(&sig) else {
                continue;
            };
            for row in rows {
                self.rows.remove(&row);
            }
        }
    }
}

impl From<BitwiseRow> for Row {
    fn from(value: BitwiseRow) -> Self {
        Self::Bitwise(value)
    }
}
