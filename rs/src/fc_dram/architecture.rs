//! Contains all architecture-specific descriptions
//! - [`FCDRAMArchitecture`] = trait which needs to be implemented for your DRAM-module
//! - [`Instruction`] = contains all instructions supported by FC-DRAM architecture
//! - [ ] `RowAddress`: utility functions to get subarray-id and row-addr within that subarray from
//!     - [ ] ->create `pub struct Architecture`
//! RowAddress (eg via bit-shifting given bitmasks for subarray-id & row-addr to put on-top of
//! RowAddress


/// TODO: merge `rows.rs` with `mod.rs` and move into `arch.rs`
use eggmock::{Id, Mig, ProviderWithBackwardEdges, Signal};
use rustc_hash::FxHashMap;
use std::collections::hash_map::Entry;
use std::fmt::{Display, Formatter};

/// Implement this trait for your specific DRAM-module to support FCDRAM-functionality
/// - contains the mapping of logical-ops to FCDRAM-Architecture (see
/// [`FCDRAMArchitecture::get_instructions_implementation_of_logic_ops`]
///
/// # Possible Changes in Future
///
/// - add trait-bound to a more general `Architecture`-trait to fit in the overall framework?
pub trait FCDRAMArchitecture {
    /// Returns vector of simultaneously activated rows when issuing `APA(r1,r2)`-cmd
    /// NOTE: this may depend on the used DRAM - see [3] for a method for reverse-engineering
    /// which rows are activated simultaneously (also see RowClone)
    fn get_simultaneously_activated_rows_of_apa_op(r1: RowAddress, r2: RowAddress) -> Vec<RowAddress>;

    /// Implements given logic operation using FCDRAM-Instructions
    /// REMINDER: for OR&AND additionall [`Instruction::FracOp`]s need to be issued to setup the
    /// reference subarray containing `reference_rows` in order to perform the given `logic_op` on
    /// the `compute_rows` inside the computation rows
    ///
    /// - [ ] TODO: for `NOT`: `reference_rows`=??? (empty or =result rows?)
    ///
    /// NOTE: `compute_rows` are expected to lay in the same subarray and `reference_rows` in one
    /// subarray adjacent to the compute subarray (!this is not checked but assumed to be true!)
    fn get_instructions_implementation_of_logic_ops(logic_op: SupportedLogicOps, compute_rows: Vec<RowAddress>, reference_rows: Vec<RowAddress>) -> Vec<Instruction> {
        todo!()
    }
}

pub type RowAddress = u64;

/// Instructions used in FC-DRAM
/// - NOT: implemented using `APA`
/// - AND/OR: implemented by (see [1] Chap6.1.2)
///     1. setting `V_{AND}`/`V_{OR}` in reference subarray and then issuing (using FracOperation
///        for storing `V_{DD}/2`)
///     2. Issue `APA(R_{REF},R_{COM})` to simultaneously activate `N` rows in reference subarray
///        and `N` rows in compute subarray
///     3. Wait for `t_{RAS}` (=overwrites activated cells in compute subarray with AND/OR-result)
///     4. Issue `PRE` to complete the operation
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Instruction {
    /// Needed for initializing neutral row in reference subarray (to set `V_{AND}`/`V_{OR}` (see
    /// [1](
    /// Implemented using AP without any extra cycles in between) (see [2])
    /// - `PRE` "interrupt the process of row activation, and prevent the sense amplifier from being enabled"
    FracOp(RowAddress),
    /// Multiple-Row Activation: `ACT R_F -> PRE -> ACT R_L -> PRE` of rows `R_F`,`R_L` for rows within
    /// different subarrays. As a result `R_L` holds the negated value of `R_F` (see Chap5.1 of
    /// PaperFunctionally Complete DRAMs
    /// Used to implement NOT directly
    APA(RowAddress,RowAddress),
}

impl Display for Instruction {
 fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let description = match self {
            Instruction::FracOp(row) => "AP(row)",
            Instruction::APA(row1,row2) => "APA({row1},{row2})",
        };
        write!(f, "{}", description)
    }
}

/// TODO: where to put logic for determining which rows are activated simultaneously given two
/// row-addresses
impl Instruction {
    /// Return Addreses of Rows which are used by this instruction (=operand-rows AND result-row)
    /// - REMINDER: although only two row-operands are given to `APA`, more rows can be/are affected due to *Simultaneous Row Activation* (see [3])
    /// TODO
    pub fn used_addresses(
        &self,
    ) -> Vec <RowAddress> {
    // ) -> impl Iterator<Item = RowAddress> {
        todo!()
        // let from = match self {
        //     Instruction::AAP(from, _) => from,
        //     Instruction::AP(op) => op,
        // }
        // .row_addresses(architecture);
        // let to = match self {
        //     Instruction::AAP(_, to) => Some(*to),
        //     _ => None,
        // }
        // .into_iter()
        // .flat_map(|addr| addr.row_addresses(architecture));
        // from.chain(to)
    }

    /// Returns all row-addresses whose values are overriden by this instruction
    /// TODO
    pub fn overridden_rows<'a>(
        &self,
    ) -> Vec <RowAddress> {
    // ) -> impl Iterator<Item = RowAddress> {
        todo!()
    }
}

/// Contains logical operations which are supported (natively) on FCDRAM-Architecture
pub enum SupportedLogicOps {
    NOT,
    AND,
    OR,
    /// implemented using AND+NOT
    NAND,
    /// implemented using OR+NOT
    NOR,
}
