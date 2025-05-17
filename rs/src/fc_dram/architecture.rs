//! Contains all architecture-specific descriptions
//! - [`FCDRAMArchitecture`] = DRAM-module-specific specific implementation of FCDRAMArchitecture
//! - [`Instruction`] = contains all instructions supported by FC-DRAM architecture
//! - [ ] `RowAddress`: utility functions to get subarray-id and row-addr within that subarray from
//! RowAddress (eg via bit-shifting given bitmasks for subarray-id & row-addr to put on-top of
//! RowAddress

use std::{fmt::{Display, Formatter}, sync::LazyLock};

/// Main variable specifying architecture of DRAM-module for which to compile for
/// - this is currently just an example implementation for testing purpose; (TODO: make this
/// configurable at runtime)
/// TODO: add field to simulate row-decoder circuitry, needed for impl Simultaneous-row-activation
/// TODO: make this configurable at runtime
static ARCHITECTURE: LazyLock<FCDRAMArchitecture> = LazyLock::new(|| {
    const NR_SUBARRAYS: i64 = 2i64.pow(7);
    const ROWS_PER_SUBARRAY: i64 = 2i64.pow(9);

    // TODO: init architecture a run-time, eg from config file
    let get_activated_rows_from_apa = |row1: RowAddress, row2: RowAddress| -> Vec<RowAddress> {
        let mut activated_rows = vec!(row1, row2);
        // TODO: get other activated rows and add them to `activated_rows`
        activated_rows.push(123456); // TEST: add random row
        activated_rows
    };

    let get_distance_of_row_to_sense_amps = |row: RowAddress| -> RowDistanceToSenseAmps {
        // ASSUMPTION: last & first rows only have sense-amps from one side
        // TODO: is this true? or do all subarrays have a line of sense-amps on both of their ends??
        let distance_to_nearest_sense_amp_in_nr_rows = if row < ROWS_PER_SUBARRAY {
            // this row is in the first subarray
            row // row-addr = distance to nearest sense-amps
        } else if  row > (NR_SUBARRAYS-1)*ROWS_PER_SUBARRAY {
            // this row is in the last subarray
            row - (NR_SUBARRAYS-1)*ROWS_PER_SUBARRAY // =distance to above sense-amps
        } else {
            // let subarray_id = row / rows_per_subarray;
            let row_nr_in_subarray = row % ROWS_PER_SUBARRAY;
            if row_nr_in_subarray < ROWS_PER_SUBARRAY {
                // row is in the 1st half of the subarray and hence nearer to the "previous" sense-amps
                row_nr_in_subarray
            } else {
                // row is in the 2nd half of the subarray and hence nearer to the "previous" sense-amps
                row_nr_in_subarray - ROWS_PER_SUBARRAY/2
            }

        };
        match distance_to_nearest_sense_amp_in_nr_rows {
            i if i < ROWS_PER_SUBARRAY / 2 / 3 => RowDistanceToSenseAmps::Close,   // 1st third  of subarray-half
            i if i < ROWS_PER_SUBARRAY / 2 / 6 => RowDistanceToSenseAmps::Middle,  // 2nd third  of subarray-half
            _ => RowDistanceToSenseAmps::Far,                                           // everything else is treated as being far away
        }
    };

    FCDRAMArchitecture {
        nr_subarrays: NR_SUBARRAYS,
        rows_per_subarray: ROWS_PER_SUBARRAY,
        get_activated_rows_from_apa,
        get_distance_of_row_to_sense_amps,
    }
});

/// - ! must be smaller than `rows_per_subarray * nr_subarrays` (this is NOT checked!)
pub type RowAddress = i64;

pub struct FCDRAMArchitecture {
    /// Nr of subarrays in a DRAM module
    nr_subarrays: i64,
    /// Nr of rows in a single subarray
    rows_per_subarray: i64,
    /// Returns all activated rows when issuing `APA(row1, row2)`
    get_activated_rows_from_apa: fn(RowAddress, RowAddress) -> Vec<RowAddress>,
    // TODO: params for calculating distance btw row and sense-amp, ... (particularly where
    // sense-amps are placed within the DRAM module ?!
    /// Given a row-addr this returns the distance of it to the sense-amps (!determinse
    /// success-rate of op using that `row` as an operand) (see [1] Chap5.2)
    /// - NOTE: Methodology used in [1] to determine distance: RowHammer
    get_distance_of_row_to_sense_amps: fn(RowAddress) -> RowDistanceToSenseAmps,
}

/// Implement this trait for your specific DRAM-module to support FCDRAM-functionality
/// - contains the mapping of logical-ops to FCDRAM-Architecture (see
/// [`FCDRAMArchitecture::get_instructions_implementation_of_logic_ops`]
///
/// # Possible Changes in Future
///
/// - add trait-bound to a more general `Architecture`-trait to fit in the overall framework?
impl FCDRAMArchitecture {

    /// Returns FC-DRAM operations to perform for each logical operation, with operand-rows NOT set !!!
    /// - addresses of row operands need to be overwritten during compilation !
    ///
    /// REMINDER: for OR&AND additional [`Instruction::FracOp`]s need to be issued to setup the
    /// reference subarray containing `reference_rows` in order to perform the given `logic_op` on
    /// the `compute_rows` inside the computation rows
    ///
    /// REMINDER: do increase the success rate of `FracOp` storing a fractional value (`V_{DD}/2`
    /// in this case, several FracOps are usually issued)
    /// - ->`FracOp`s are replicated during compilation as necessary, this is done during compilation
    ///
    /// NOTE: `compute_rows` are expected to lay in the same subarray and `reference_rows` in one
    /// subarray adjacent to the compute subarray (!this is not checked but assumed to be true!)
    fn get_instructions_implementation_of_logic_ops(logic_op: SupportedLogicOps) -> Vec<Instruction> {
        match logic_op {
            SupportedLogicOps::NOT => vec!(Instruction::APA(-1, -1)),
            SupportedLogicOps::AND => vec!(Instruction::FracOp(-1), Instruction::APA(-1, -1)),
            SupportedLogicOps::OR => vec!(Instruction::FracOp(-1), Instruction::APA(-1, -1)),
            SupportedLogicOps::NAND => {
                // 1. AND, 2. NOT
                FCDRAMArchitecture::get_instructions_implementation_of_logic_ops(SupportedLogicOps::AND)
                    .into_iter()
                    .chain( FCDRAMArchitecture::get_instructions_implementation_of_logic_ops(SupportedLogicOps::NOT))
                    .collect()
            },
            SupportedLogicOps::NOR => {
                // 1. OR, 2. NOT
                FCDRAMArchitecture::get_instructions_implementation_of_logic_ops(SupportedLogicOps::OR)
                    .into_iter()
                    .chain( FCDRAMArchitecture::get_instructions_implementation_of_logic_ops(SupportedLogicOps::NOT))
                    .collect()
            }
        }
    }
}

/// Categories of distances of rows to sense-amops
pub enum RowDistanceToSenseAmps {
    Close,
    Middle,
    Far,
}

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
            Instruction::FracOp(row) => format!("AP({row})"),
            Instruction::APA(row1,row2) => format!("APA({row1},{row2})"),
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
/// - see [`FCDRAMArchitecture::get_instructions_implementation_of_logic_ops`] for how these
/// logic-ops are mapped to FCDRAM-instructions
pub enum SupportedLogicOps {
    NOT,
    AND,
    OR,
    /// implemented using AND+NOT
    NAND,
    /// implemented using OR+NOT
    NOR,
}

/// Implements behavior of the RowDecoderCircuitry as described in [3]
/// TODO: remove in favor of passing arbitrary closure to [`FCDRAMArchitecture::get_activated_rows_from_apa`]
pub trait RowDecoder {
    /// Returns vector of simultaneously activated rows when issuing `APA(r1,r2)`-cmd
    /// NOTE: this may depend on the used DRAM - see [3] for a method for reverse-engineering
    /// which rows are activated simultaneously (also see RowClone)
    fn get_simultaneously_activated_rows_of_apa_op(&self, r1: RowAddress, r2: RowAddress) -> Vec<RowAddress>;

    // TODO: get activation pattern for given rows r1,r2 (N:N vs N:2N) - or just check whether
    // N:2N: is supported and let `get_simultaneously_activated_rows_of_apa_op()` handle the rest?
}

