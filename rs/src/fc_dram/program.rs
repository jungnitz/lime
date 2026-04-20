//! Functionality for generating actual program using architecture defined in [`architecture`] by
//! compiling given logic-network (see [`compilation`]) and potentially adding some manual
//! optimizations ([`optimization`])
use super::architecture::RowAddress;
use crate::fc_dram::architecture::{Instruction, ROW_ID_BITMASK};
use eggmock::Signal;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};


#[derive(Debug, Clone)]
pub struct Program {
    pub instructions: Vec<Instruction>,
    /// Specifies in which rows constants have to be placed (!have to be placed in EVERY subarray)
    /// - TODO: adjust this to only place in subarrays which are actually used as reference subarrays during program execution
    pub constants_row_placement: HashMap<usize, RowAddress>,
    /// Specifies where row-operands should be placed prior to calling this program
    /// (This is a convention which tells the user of this lib where the data should be placed within the DRAM before executing this program)
    /// - NOTE: Signals might have to be placed in several subarrays (REMINDER: movement in btw subarrays is not supported by FCDRAM)
    pub input_row_operands_placement: HashMap<Signal, Vec<RowAddress>>,
    /// Specifies into which rows output-operands will have been placed after the program has run successfully
    pub output_row_operands_placement: HashMap<Signal, RowAddress>,
}

impl Program {
    pub fn new(instructions: Vec<Instruction>) -> Self {
        Self {
            instructions,
            constants_row_placement: HashMap::new(),
            input_row_operands_placement: HashMap::new(),
            output_row_operands_placement: HashMap::new(),
        }
    }
}

/// Print the generated program in human-readable form
impl Display for Program {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let display_row = |row: &RowAddress| {
            format!("{}.{}", row.get_subarray_id(), row.0 & ROW_ID_BITMASK)
        }; // display subarray separately


        let display_rows = |rows: Vec<RowAddress>| {
            let formatted: Vec<String> = rows.iter()
                .map(|&row| format!("{}.{}", row.get_subarray_id(), row.0 & ROW_ID_BITMASK))
                .collect();

            format!("[{}]", formatted.join(", "))
        }; // display subarray separately

        writeln!(f, "---------------------------------------")?;
        writeln!(f, "Input operand placement:")?;
        for (signal, rows) in &self.input_row_operands_placement {
            writeln!(f, "{:?} in {}", signal, display_rows(rows.to_vec()))?;
        }
        writeln!(f, "---------------------------------------")?;
        writeln!(f, "Constant operand placement:")?;
        for (constant, row) in &self.constants_row_placement {
            writeln!(f, "{} in {}", constant, display_row(&row.local_rowaddress_to_subarray_id(super::architecture::SubarrayId(0))))?;
        }
        writeln!(f, "---------------------------------------")?;


        for instr in &self.instructions {
            writeln!(f, "{}", instr)?;
        }

        writeln!(f, "---------------------------------------")?;
        writeln!(f, "Output operand placement:")?;
        for (signal, row) in &self.output_row_operands_placement{
            writeln!(f, "{:?} in {}", signal, display_row(row))?;
        }
        writeln!(f, "---------------------------------------")?;
        Ok(())
    }
}
