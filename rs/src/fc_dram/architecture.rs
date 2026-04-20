//! Contains all architecture-specific descriptions
//! - [`FCDRAMArchitecture`] = DRAM-module-specific specific implementation of FCDRAMArchitecture
//! - [`Instruction`] = contains all instructions supported by FC-DRAM architecture
//! - [ ] `RowAddress`: utility functions to get subarray-id and row-addr within that subarray from
//!
//! RowAddress (eg via bit-shifting given bitmasks for subarray-id & row-addr to put on-top of RowAddress

use std::{cmp::Ordering, collections::{HashMap, HashSet}, fmt::{self, Display, Formatter}, ops, sync::LazyLock};
use serde::{Deserialize, Serialize};
use strum_macros::EnumIter;

pub const NR_SUBARRAYS: u64 = 2u64.pow(7);
pub const ROWS_PER_SUBARRAY: u64 = 2u64.pow(9);
pub const SUBARRAY_ID_BITMASK: u64 = 0b1_111_111_000_000_000; // 7 highest bits=subarray id
pub const ROW_ID_BITMASK: u64 = 0b0_000_000_111_111_111; // 7 highest bits=subarray id

// some utility functions
pub fn subarrayid_to_subarray_address(subarray_id: SubarrayId) -> RowAddress {
    RowAddress(subarray_id.0 << ROWS_PER_SUBARRAY.ilog2()) // lower bits=rows in subarray
}

/// All Subarrays (except the ones at the edges) have two neighboring subarrays: one below (subarray_id+1) and one above (subarray_id-1)
/// - currently the following subarrays are used together for computations: 0&1,2&3,4&5,..
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, EnumIter, Serialize, Deserialize)]
pub enum NeighboringSubarrayRelPosition {
    /// `subarray_id-1`
    Above,
    /// `subarray_id+1`
    Below,
}

impl NeighboringSubarrayRelPosition {
    /// Get whether `subarray1` is above or below `relative_to`
    pub fn get_relative_position(subarray: &SubarrayId, relative_to: &SubarrayId) -> Self {
        assert!((subarray.0 as isize - relative_to.0 as isize).abs() == 1, "Given Arrays are not neighboring arrays");
        if subarray.0 > relative_to.0 {
            NeighboringSubarrayRelPosition::Below
        } else {
            NeighboringSubarrayRelPosition::Above
        }
    }
}

/// Main variable specifying architecture of DRAM-module for which to compile for
/// - this is currently just an example implementation for testing purpose; (TODO: make this configurable at runtime)
///
/// TODO: add field to simulate row-decoder circuitry, needed for impl Simultaneous-row-activation
/// TODO: make this configurable at runtime
pub static ARCHITECTURE: LazyLock<FCDRAMArchitecture> = LazyLock::new(|| {

    let mut row_activated_by_rowaddress_tuple: HashMap<RowAddress, HashSet<(RowAddress, RowAddress)>> = HashMap::new(); // for each row store which RowAddress-combinations activate it

    // Implementation of the Hypothetical Row Decoder from [3] Chap4.2
    // - GWLD (Global Wordline Decoder)=decode higher bits to select addressed subarray
    // - LWLD (Local Wordline Decoder)=hierarchy of decoders which decode lower bits; latches remain set when using `APA`
    //      - see [3] Chap4.2: nr of Predecoders in LWLD determines number & addresses of simultaneously activated rows
    //  - does work for the example shown in [3] Chap3.2: `APA(256,287)` activates rows `287,286,281,280,263,262,257,256`
    // TODO: add overlapping of higher-order-bits (GWLD)
    //      - at the moment high-order bits (=subarray-id) needs to be added manually using eg `subarrayid_to_subarray_address()` helper function
    // TODO: maybe evaluate statically?
    let get_activated_rows_from_apa = |row1: RowAddress, row2: RowAddress| -> Vec<RowAddress> {
        // 1. Define Predecoders by defining for which of the bits they're responsible
        // each Predecoder is resonsible for some of the lower order bits
        let predecoder_bitmasks = [
            0b110000000, // first predecoder (PE) predecodes bits[8,7]
            0b001100000, // Predecoder PD
            0b000011000, // Predecoder PC
            0b000000110, // Predecoder PB
            0b000000001, // last predecoder (PA) predecodes bits[0]
        ];

        // for each predecoder store which bits will remain set due to `APA(row1,row)`:
        let overlapping_bits = [
            // latches set by `ACT(row1)`  --- latches set by `ACT(row2)`
            [ row1.0 & predecoder_bitmasks[0], row2.0 & predecoder_bitmasks[0]],
            [ row1.0 & predecoder_bitmasks[1], row2.0 & predecoder_bitmasks[1]],
            [ row1.0 & predecoder_bitmasks[2], row2.0 & predecoder_bitmasks[2]],
            [ row1.0 & predecoder_bitmasks[3], row2.0 & predecoder_bitmasks[3]],
            [ row1.0 & predecoder_bitmasks[4], row2.0 & predecoder_bitmasks[4]],
        ];

        let mut activated_rows  = vec!();       // TODO: get other activated rows and add them to `activated_rows`
        // compute all simultaneously activated rows
        for i in 0..1 << predecoder_bitmasks.len() {
            let activated_row = overlapping_bits.iter()
                // start with all row-address bits unset (=0) and first predecoder stage (=1)
                .fold((RowAddress(0), 1), |(row, predecoder_stage_onehot), new_row_bits|{
                    let bitmask_to_choose = (i & predecoder_stage_onehot) > 0;
                    (RowAddress(row.0 | new_row_bits[bitmask_to_choose as usize]), predecoder_stage_onehot << 1)
                });
            activated_rows.push(activated_row.0);
        }
        // debug!("`APA({row1},{row2})` activates the following rows simultaneously: {activated_rows:?}");
        activated_rows.dedup(); // no need for `.unique()` since this implementation adds equivalent RowAddresses one after the other (!check!!)
                                 // NOTE: works in-place
        // remove duplicate entries
        activated_rows.into_iter().collect::<HashSet<_>>().into_iter().collect()
    };

    // just a dummy implementation, see [5] Chap3.2 for details why determining the distance based on the Row Addresses issued by the MemController is difficult
    let get_distance_of_row_to_sense_amps = |row: RowAddress, subarray_rel_position: NeighboringSubarrayRelPosition| -> RowDistanceToSenseAmps {
        // NOTE: last & first subarrays only have sense-amps from one side
        if (row.get_subarray_id().0 == NR_SUBARRAYS-1 && subarray_rel_position == NeighboringSubarrayRelPosition::Below) || (row.get_subarray_id().0 == 0 && subarray_rel_position == NeighboringSubarrayRelPosition::Above) {
            panic!("Edge subarrays have sense-amps only connected from one side");
        }

        let local_row_address= RowAddress(row.0 & ROW_ID_BITMASK);

        let distance_to_above_subarray = match local_row_address {
            i if i.0 < ROWS_PER_SUBARRAY / 2 / 3 => RowDistanceToSenseAmps::Close,   // 1st third  of subarray-half
            i if i.0 < ROWS_PER_SUBARRAY / 2 / 6 => RowDistanceToSenseAmps::Middle,  // 2nd third  of subarray-half
            _ => RowDistanceToSenseAmps::Far,                                           // everything else is treated as being far away
        };

        match subarray_rel_position {
            NeighboringSubarrayRelPosition::Above => distance_to_above_subarray,
            NeighboringSubarrayRelPosition::Below => distance_to_above_subarray.reverse(), // rows close to above subarray are far from below subarray etc
        }
    };

    // precompute things based on given SRA (simultaneous row activation function)
    let mut precomputed_simultaneous_row_activations = HashMap::new();
    for i in 0..ROWS_PER_SUBARRAY {
        precomputed_simultaneous_row_activations.insert((RowAddress(i),RowAddress(i)), vec!(RowAddress(i))); // special case: no other row is activated when executing `APA(r1,r1)`
        for j in i+1..ROWS_PER_SUBARRAY {
            let activated_rows = get_activated_rows_from_apa(RowAddress(i), RowAddress(j));
            precomputed_simultaneous_row_activations.insert((RowAddress(i),RowAddress(j)), activated_rows.clone());
            precomputed_simultaneous_row_activations.insert((RowAddress(j),RowAddress(i)), activated_rows.clone());

            for row in activated_rows {
                row_activated_by_rowaddress_tuple.entry(row)
                    .or_default()
                    .insert((RowAddress(i),RowAddress(j)));
            }
        }
    }
    // debug!("Precomputed SRAs: {:#?}", precomputed_simultaneous_row_activations.iter().take(20).collect::<Vec<_>>());

    let sra_degree_to_rowaddress_combinations=  precomputed_simultaneous_row_activations.iter()
        .fold(HashMap::new(), |mut acc: HashMap<SupportedNrOperands, Vec<(RowAddress,RowAddress)>>, (row_combi, activated_rows)| {
            acc.entry(SupportedNrOperands::try_from(activated_rows.len() as u8).unwrap()).or_default().push(*row_combi);
            acc
    });
    // output how many combinations of row-addresses activate the given nr of rows
    // debug!("SRAs row-nr to row-addr mapping: {:#?}", sra_degree_to_rowaddress_combinations.iter().map(|(k,v)| format!("{k} rows activated in {} addr-combinations", v.len())).collect::<Vec<String>>());

    FCDRAMArchitecture {
        nr_subarrays: NR_SUBARRAYS,
        rows_per_subarray: ROWS_PER_SUBARRAY,
        get_activated_rows_from_apa,
        precomputed_simultaneous_row_activations,
        row_activated_by_rowaddress_tuple,
        sra_degree_to_rowaddress_combinations,
        get_distance_of_row_to_sense_amps,
    }
});

/// - ! must be smaller than `rows_per_subarray * nr_subarrays` (this is NOT checked!)
#[derive(Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RowAddress(pub u64);

impl RowAddress {
    /// Return subarray-id the row lies in
    pub fn get_subarray_id(&self) -> SubarrayId {
        SubarrayId((self.0 & SUBARRAY_ID_BITMASK) >> ROWS_PER_SUBARRAY.ilog2())
    }

    /// Converts RowAddress to the same row address but in the other subarray
    pub fn local_rowaddress_to_subarray_id(&self, subarray_id: SubarrayId) -> RowAddress {
        let local_row_address = self.0 & ROW_ID_BITMASK;
        RowAddress( local_row_address | subarrayid_to_subarray_address(subarray_id).0 )
    }
}

impl fmt::Display for RowAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}", self.get_subarray_id().0, self.0 & ROW_ID_BITMASK)
    }
}

impl fmt::Debug for RowAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}", self.get_subarray_id().0, self.0 & ROW_ID_BITMASK)
    }
}

impl From<u64> for RowAddress {
    fn from(value: u64) -> Self {
        RowAddress(value)
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct SubarrayId(pub u64);

impl SubarrayId {
    /// Currently all ops only work on half of the cells (every 2nd cell) with two subarrays being
    /// in a compute/reference subarray "relation" with exactly one other neighboring subarray.
    /// This function returns that other partner (compute/reference) subarray
    pub fn get_partner_subarray(&self) -> Self {
        if self.0 % 2 == 0 {
            SubarrayId(self.0 + 1)
        } else {
            SubarrayId(self.0 - 1)
        }
    }
}

impl fmt::Display for SubarrayId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct SuccessRate(pub f64);

impl SuccessRate {
    pub fn new(success_rate: f64) -> Self {
        if (0.0..=1.0).contains(&success_rate) {
            SuccessRate(success_rate)
        } else {
            panic!("SuccessRate must in [0,1], but was {success_rate}");
        }
    }
}

impl Eq for SuccessRate {}

impl PartialOrd for SuccessRate {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other)) // delegate to total_cmp
    }
}

impl Ord for SuccessRate {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.total_cmp(&other.0)
    }
}

impl ops::Mul<SuccessRate> for SuccessRate {
    type Output = SuccessRate;

    fn mul(self, rhs: Self) -> Self::Output {
        SuccessRate::new(rhs.0 * self.0)
    }
}

impl From<f64> for SuccessRate {
    fn from(val: f64) -> Self {
        SuccessRate::new(val)
    }
}

/// TODO: add field encoding topology of subarrays (to determine which of them share sense-amps)
pub struct FCDRAMArchitecture {
    /// Nr of subarrays in a DRAM module
    pub nr_subarrays: u64,
    /// Nr of rows in a single subarray
    pub rows_per_subarray: u64,
    /// Returns all activated rows when issuing `APA(row1, row2)`
    /// - NOTE: `row1`,`row2` are expected to reside in adjacent subarrays
    /// - NOTE: the simultaneously activated rows are expected to have the same addresses in both subarrays
    ///     - eg `APA(11,29)` (with 1st digit=subarray-id, 2nd digit=row-id) could simultaneously activate rows `0,1,7,9` in subarray1 and subarray2
    get_activated_rows_from_apa: fn(RowAddress, RowAddress) -> Vec<RowAddress>,
    /// Stores which rows are simultaneously activated for each combination of Row-Addresses (provided to `APA`-operation)
    /// - REASON: getting the simultaneously activated will probably be requested very frequently (time-space tradeoff, rather than recomputing on every request))
    /// - REMEMBER: set `subarrayid` of passed row-addresses to 0 (activated rows are precomputed exemplary for RowAddresses in subarray=0 since activated rows do <u>not</u> depend on corresponding subarrays)
    pub precomputed_simultaneous_row_activations: HashMap<(RowAddress, RowAddress), Vec<RowAddress>>,
    /// Map degree of SRA (=nr of activated rows by that SRA) to all combinations of RowAddresses which have that degree of SRA
    /// - use to eg restrict the choice of row-addresses for n-ary AND/OR (eg 4-ary AND -> at least activate 8 rows; more rows could be activated when using input replication)
    /// NOTE: LogicOp determiens success-rate
    pub sra_degree_to_rowaddress_combinations: HashMap<SupportedNrOperands, Vec<(RowAddress,RowAddress)>>,
    // pub sra_degree_to_rowaddress_combinations: HashMap<(u8, LogicOp), BTreeMap<(RowAddress,RowAddress), SuccessRate>>, // to large runtime-overhead :/
    /// Stores for every rows which combinations of RowAddresses activate that row (needed for finding appropriate safe space rows)
    pub row_activated_by_rowaddress_tuple: HashMap<RowAddress, HashSet<(RowAddress, RowAddress)>>,
    /// Given a row-addr this returns the distance of it to the sense-amps (!determinse success-rate of op using that `row` as an operand) (see [1] Chap5.2)
    /// - NOTE: a realistic implementation should use the Methodology from [1] to determine this distance (RowHammer)
    ///     - there is no way of telling the distance of a row without testing manually (see [5] Chap3.2: "consecutive row addresses issued by the memory controller can be mapped to entirely different regions of DRAM")
    pub get_distance_of_row_to_sense_amps: fn(RowAddress, NeighboringSubarrayRelPosition) -> RowDistanceToSenseAmps,
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
    pub fn get_instructions_implementation_of_logic_ops(logic_op: LogicOp) -> Vec<Instruction> {
        match logic_op {
            LogicOp::NOT => vec!(Instruction::ApaNOT(RowAddress(0), RowAddress(0))),
            LogicOp::AND => vec!(Instruction::FracOp(RowAddress(0)), Instruction::ApaNOT(RowAddress(0), RowAddress(0))),
            LogicOp::OR => vec!(Instruction::FracOp(RowAddress(0)), Instruction::ApaNOT(RowAddress(0), RowAddress(0))),
            LogicOp::NAND => {
                // 1. AND, 2. NOT
                FCDRAMArchitecture::get_instructions_implementation_of_logic_ops(LogicOp::AND)
                    .into_iter()
                    .chain( FCDRAMArchitecture::get_instructions_implementation_of_logic_ops(LogicOp::NOT))
                    .collect()
            },
            LogicOp::NOR => {
                // 1. OR, 2. NOT
                FCDRAMArchitecture::get_instructions_implementation_of_logic_ops(LogicOp::OR)
                    .into_iter()
                    .chain( FCDRAMArchitecture::get_instructions_implementation_of_logic_ops(LogicOp::NOT))
                    .collect()
            }
        }
    }

    fn get_activated_rows() {

    }
}

/// Categories of distances of rows to sense-amops
/// - HIB (higher is better): that's why [`RowDistanceToSenseAmps::Close`] has the highest int value
#[derive(Hash,Eq,PartialEq,PartialOrd,Ord)]
pub enum RowDistanceToSenseAmps {
    Close=2,
    Middle=1,
    Far=0,
}

impl RowDistanceToSenseAmps {
    /// Reverse distance (Far-> Close, Middle -> Middle, Close->Far), useful when row's distance to other neighboring subarray (below/above) is needed
    pub fn reverse(&self) -> Self {
        match &self {
            RowDistanceToSenseAmps::Close => RowDistanceToSenseAmps::Far,
            RowDistanceToSenseAmps::Middle=> RowDistanceToSenseAmps::Middle,
            RowDistanceToSenseAmps::Far=> RowDistanceToSenseAmps::Close,
        }
    }
}

type Comment = String;
/// Instructions used in FC-DRAM
/// - NOT: implemented using `APA`
/// - AND/OR: implemented by (see [1] Chap6.1.2)
///     1. setting `V_{AND}`/`V_{OR}` in reference subarray and then issuing (using FracOperation
///        for storing `V_{DD}/2`)
///     2. Issue `APA(R_{REF},R_{COM})` to simultaneously activate `N` rows in reference subarray
///        and `N` rows in compute subarray
///     3. Wait for `t_{RAS}` (=overwrites activated cells in compute subarray with AND/OR-result)
///     4. Issue `PRE` to complete the operation
///
/// Additionally RowClone-operations are added for moving data around if needed (eg if valid data
/// would be affected by following Simultaneous-Row-Activations)
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Instruction {
    /// Needed for initializing neutral row in reference subarray (to set `V_{AND}`/`V_{OR}` (see
    /// [1](
    /// Implemented using AP without any extra cycles in between) (see [2])
    /// - `PRE` "interrupt the process of row activation, and prevent the sense amplifier from being enabled"
    FracOp(RowAddress),
    /// Multiple-Row Activation: `ACT R_F -> PRE -> ACT R_L -> PRE` of rows `R_F`,`R_L` for rows within
    /// different subarrays. As a result `R_L` holds the negated value of `R_F` (see Chap5.1 of PaperFunctionally Complete DRAMs
    /// src=1st operand, dst=2nd operand
    ApaNOT(RowAddress,RowAddress),
    /// Multiple-Row Activation: `ACT R_F -> PRE -> ACT R_L -> PRE` of rows `R_F`,`R_L` for rows within
    /// different subarrays (but with different timings than `ApaNOT`!)
    /// src=1st operand, dst=2nd operand
    ApaAndOr(RowAddress,RowAddress),
    /// Fast-Parallel-Mode RowClone for cloning row-data within same subarray
    /// - corresponds to `AA`, basically copies from src-row -> row-buffer -> dst-row
    /// - first operand=src, 2nd operand=dst where `src` and `dst` MUST reside in the same subarray !
    ///
    /// Comment indicates what this FPM was issued for (for simpler debugability)
    RowCloneFPM(RowAddress, RowAddress, Comment),
}

impl Display for Instruction {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let display_row = |row: &RowAddress| { format!("{}.{}", row.get_subarray_id().0, row.0 & ROW_ID_BITMASK)}; // display subarray separately
        // TODO: change string-representation to display subarray-id
        let description = match self {
            Instruction::FracOp(row) => format!("AP({})", display_row(row)),
            Instruction::ApaNOT(row1,row2) => format!("APA_NOT({},{})", display_row(row1), display_row(row2)),
            Instruction::ApaAndOr(row1,row2) => {
                let (src_array, dst_array) = (row1.get_subarray_id(), row2.get_subarray_id());
                let activated_rows: Vec<RowAddress> =  ARCHITECTURE.precomputed_simultaneous_row_activations.get(&(row1.local_rowaddress_to_subarray_id(SubarrayId(0)),row2.local_rowaddress_to_subarray_id(SubarrayId(0)))).unwrap()
                    .iter().flat_map(|row| vec!(row.local_rowaddress_to_subarray_id(src_array), row.local_rowaddress_to_subarray_id(dst_array)))
                    .collect();
                format!("APA_AND_OR({},{}) // activates {:?}", display_row(row1), display_row(row2), activated_rows)
            },
            Instruction::RowCloneFPM(row1, row2, comment) => format!("AAP({},{}) // {}", display_row(row1), display_row(row2), comment),
        };
        write!(f, "{}", description)
    }
}

/// TODO: where to put logic for determining which rows are activated simultaneously given two
/// row-addresses
impl Instruction {

    /// TODO: rewrite this (eg `ApaNOT` and `ApaAndOr` take different amount of time !!, see Figure
    pub fn get_nr_memcycles(&self) -> u16 {
        match self {
            Instruction::FracOp(__) => 7,     // see [2] ChapIII.A, (two cmd-cycles + five idle cycles)
            // TODO: change to ns (t_{RAS}+6ns) - `t_{RAS}` to mem cycles
            Instruction::ApaNOT(_, _) => 3,            // NOTE: this is not explicitly written in the paper, TODO: check with authors
            Instruction::ApaAndOr(_, _) => 3,            // NOTE: this is not explicitly written in the paper, TODO: check with authors
            Instruction::RowCloneFPM(_, _, _) => 2,    // see [4] Chap3.2 (TODO: not correct, given as 90ns?)
        }
    }

    /// Success Rate of instructions depends on:
    /// - for AND/OR (`APA`): number of input operands (see [1] Chap6.3)
    ///     - data pattern can't be taken into consideration here since its not known at compile-time (unknown at compile-time)
    ///     - as well as temperature and DRAM speed rate (ignored here)
    ///
    /// TAKEAWAY: `OR` is more reliable than `AND`
    pub fn get_success_rate_of_apa(&self, implemented_op: LogicOp) -> SuccessRate {

        // Quote from [1] Chap6.3: "the distance of all simultaneously activated rows" - unclear how this classification happend exactly. Let's be conservative and assume the worst-case behavior
        // (furthest away row for src-operands). For dst-rows we use the one closest to the sense-amps, since we can choose from which of the rows to read/save the result form

        let success_rate_by_row_distance = implemented_op.get_success_rate_by_row_distance();

        // include nr of operands and distance of rows to sense-amps into success-rate
        match self {
            Instruction::ApaNOT( src, dst) => {
                let activated_rows = ARCHITECTURE.precomputed_simultaneous_row_activations.get(&(*src,*dst)).expect("[ERR] Missing SRA for ({r1},{r2}");
                let nr_operands = activated_rows.len(); // ASSUMPTION: it seems like "operands" referred to the number of activated rows (see [1]
                // taken from [1] Chap6.3
                let success_rate_per_operandnr = HashMap::from([
                    (2, 94.94),
                    (4, 94.94),
                    (8, 95.85),
                    (16, 95.87),
                    (32, 0.000) // no value in paper ://
                ]);
                // nr_operand_success_rate.get(&nr_operands);

                let (src_array, dst_array) = (src.get_subarray_id(), dst.get_subarray_id());
                let furthest_src_row = activated_rows.iter()
                    .map(|row| (ARCHITECTURE.get_distance_of_row_to_sense_amps)(*row, NeighboringSubarrayRelPosition::get_relative_position(&src_array, &dst_array))) // RowDistanceToSenseAmps::Far; // TODO: get this
                    .max()
                    .expect("[ERR] Activated rows were empty");
                // NOTE: SRA is assumed to activate the same row-addresses in both subarrays
                let closest_dst_row = activated_rows.iter()
                    .map(|row| (ARCHITECTURE.get_distance_of_row_to_sense_amps)(*row, NeighboringSubarrayRelPosition::get_relative_position(&dst_array, &src_array))) // RowDistanceToSenseAmps::Far; // TODO: get this
                    .min()
                    .expect("[ERR] Activated rows were empty");
                let total_success_rate = *success_rate_per_operandnr.get(&nr_operands).expect("[ERR] {nr_operands} not =1|2|4|8|16, the given SRA function seems to not comply with this core assumption.")
                    * success_rate_by_row_distance.get(&(furthest_src_row, closest_dst_row)).unwrap().0;
                SuccessRate::new(total_success_rate)
            },
            _ => SuccessRate::new(1.0),
        }
    }
}

/// Contains logical operations which are supported (natively) on FCDRAM-Architecture
/// - see [`FCDRAMArchitecture::get_instructions_implementation_of_logic_ops`] for how these logic-ops are mapped to FCDRAM-instructions
#[derive(Debug, Clone, Copy, Eq, Hash, PartialEq, EnumIter)]
pub enum LogicOp {
    NOT,
    AND,
    OR,
    /// implemented using AND+NOT
    NAND,
    /// implemented using OR+NOT
    NOR,
}

impl LogicOp {

    /// see [1] Chap5.3 and Chap6.3
    pub fn get_success_rate_by_row_distance(&self) -> HashMap<(RowDistanceToSenseAmps,RowDistanceToSenseAmps), SuccessRate> {
        match self {
            LogicOp::NOT => HashMap::from([
                    // ((src,dst), success_rate)
                    ((RowDistanceToSenseAmps::Close,RowDistanceToSenseAmps::Close), 51.71.into()),
                    ((RowDistanceToSenseAmps::Middle,RowDistanceToSenseAmps::Close), 54.93.into()),
                    ((RowDistanceToSenseAmps::Far,RowDistanceToSenseAmps::Close), 44.16.into()),
                    ((RowDistanceToSenseAmps::Close,RowDistanceToSenseAmps::Middle), 57.47.into()),
                    ((RowDistanceToSenseAmps::Middle,RowDistanceToSenseAmps::Middle), 53.47.into()),
                    ((RowDistanceToSenseAmps::Far,RowDistanceToSenseAmps::Middle), 81.92.into()),
                    ((RowDistanceToSenseAmps::Close,RowDistanceToSenseAmps::Far), 45.34.into()),
                    ((RowDistanceToSenseAmps::Middle,RowDistanceToSenseAmps::Far), 85.02.into()),
                    ((RowDistanceToSenseAmps::Far,RowDistanceToSenseAmps::Far), 75.13.into()),
                ]),
            LogicOp::AND => HashMap::from([
                    // ((reference,compute), success_rate)
                    ((RowDistanceToSenseAmps::Close,RowDistanceToSenseAmps::Close), 98.81.into()),
                    ((RowDistanceToSenseAmps::Middle,RowDistanceToSenseAmps::Close), 99.20.into()),
                    ((RowDistanceToSenseAmps::Far,RowDistanceToSenseAmps::Close), 80.04.into()),
                    ((RowDistanceToSenseAmps::Close,RowDistanceToSenseAmps::Middle), 97.08.into()),
                    ((RowDistanceToSenseAmps::Middle,RowDistanceToSenseAmps::Middle), 83.26.into()),
                    ((RowDistanceToSenseAmps::Far,RowDistanceToSenseAmps::Middle), 97.71.into()),
                    ((RowDistanceToSenseAmps::Close,RowDistanceToSenseAmps::Far), 75.84.into()),
                    ((RowDistanceToSenseAmps::Middle,RowDistanceToSenseAmps::Far), 95.29.into()),
                    ((RowDistanceToSenseAmps::Far,RowDistanceToSenseAmps::Far), 94.95.into()),
                ]),
            LogicOp::OR => HashMap::from([
                    // ((reference,compute), success_rate)
                    ((RowDistanceToSenseAmps::Close,RowDistanceToSenseAmps::Close), 99.51.into()),
                    ((RowDistanceToSenseAmps::Middle,RowDistanceToSenseAmps::Close), 99.65.into()),
                    ((RowDistanceToSenseAmps::Far,RowDistanceToSenseAmps::Close), 94.29.into()),
                    ((RowDistanceToSenseAmps::Close,RowDistanceToSenseAmps::Middle), 98.98.into()),
                    ((RowDistanceToSenseAmps::Middle,RowDistanceToSenseAmps::Middle), 94.15.into()),
                    ((RowDistanceToSenseAmps::Far,RowDistanceToSenseAmps::Middle), 98.95.into()),
                    ((RowDistanceToSenseAmps::Close,RowDistanceToSenseAmps::Far), 89.23.into()),
                    ((RowDistanceToSenseAmps::Middle,RowDistanceToSenseAmps::Far), 98.59.into()),
                    ((RowDistanceToSenseAmps::Far,RowDistanceToSenseAmps::Far), 98.80.into()),
                ]),
            LogicOp::NAND => HashMap::from([
                    // ((reference,compute), success_rate)
                    ((RowDistanceToSenseAmps::Close,RowDistanceToSenseAmps::Close), 98.81.into()),
                    ((RowDistanceToSenseAmps::Middle,RowDistanceToSenseAmps::Close), 99.20.into()),
                    ((RowDistanceToSenseAmps::Far,RowDistanceToSenseAmps::Close), 79.59.into()),
                    ((RowDistanceToSenseAmps::Close,RowDistanceToSenseAmps::Middle), 97.08.into()),
                    ((RowDistanceToSenseAmps::Middle,RowDistanceToSenseAmps::Middle), 82.98.into()),
                    ((RowDistanceToSenseAmps::Far,RowDistanceToSenseAmps::Middle), 97.67.into()),
                    ((RowDistanceToSenseAmps::Close,RowDistanceToSenseAmps::Far), 75.50.into()),
                    ((RowDistanceToSenseAmps::Middle,RowDistanceToSenseAmps::Far), 95.19.into()),
                    ((RowDistanceToSenseAmps::Far,RowDistanceToSenseAmps::Far), 94.95.into()),
                ]),
            LogicOp::NOR => HashMap::from([
                    // ((reference,compute), success_rate)
                    ((RowDistanceToSenseAmps::Close,RowDistanceToSenseAmps::Close), 99.51.into()),
                    ((RowDistanceToSenseAmps::Middle,RowDistanceToSenseAmps::Close), 99.65.into()),
                    ((RowDistanceToSenseAmps::Far,RowDistanceToSenseAmps::Close), 94.09.into()),
                    ((RowDistanceToSenseAmps::Close,RowDistanceToSenseAmps::Middle), 98.97.into()),
                    ((RowDistanceToSenseAmps::Middle,RowDistanceToSenseAmps::Middle), 94.03.into()),
                    ((RowDistanceToSenseAmps::Far,RowDistanceToSenseAmps::Middle), 98.90.into()),
                    ((RowDistanceToSenseAmps::Close,RowDistanceToSenseAmps::Far), 89.15.into()),
                    ((RowDistanceToSenseAmps::Middle,RowDistanceToSenseAmps::Far), 98.52.into()),
                    ((RowDistanceToSenseAmps::Far,RowDistanceToSenseAmps::Far), 98.80.into()),
                ]),
            }
    }

    /// taken from Figure7 (NOT) & Figure15 (AND/OR/NAND/NOR) in [1] (using Mean-dot)
    /// - NOTE: since the values have been read from the diagram they might differ +-3% from the actually measured values
    ///     - remeasuring the values on own setup might be beneficial here !
    ///
    /// In General:
    /// - for AND/OR/NAND/NOR: "The success rate of bitwise operations consistently increases as the number of input operands increases." (see Observation 11 [1])
    /// - for NOT: seems to be the opposite
    pub fn get_success_rate_by_nr_operands(&self) -> HashMap<SupportedNrOperands, SuccessRate> {
       match self {
            LogicOp::NOT => HashMap::from([
                    // ((src,dst), success_rate)
                    (SupportedNrOperands::One, 98.5.into()),
                    (SupportedNrOperands::Two, 97.5.into()),
                    (SupportedNrOperands::Four, 97.0.into()),
                    (SupportedNrOperands::Eight, 28.0.into()),
                    (SupportedNrOperands::Sixteen, 10.0.into()),
                    (SupportedNrOperands::Thirtytwo, 8.0.into()),
                ]),
            LogicOp::AND => HashMap::from([
                    // ((reference,compute), success_rate)
                    (SupportedNrOperands::Two, 86.0.into()),
                    (SupportedNrOperands::Four, 91.5.into()),
                    (SupportedNrOperands::Eight, 92.5.into()),
                    (SupportedNrOperands::Sixteen, 96.0.into()),
                ]),
            LogicOp::OR => HashMap::from([
                    // ((reference,compute), success_rate)
                    // TODO
                    (SupportedNrOperands::Two, 97.5.into()),
                    (SupportedNrOperands::Four, 97.0.into()),
                    (SupportedNrOperands::Eight, 28.0.into()),
                    (SupportedNrOperands::Sixteen, 10.0.into()),
                ]),
            LogicOp::NAND => HashMap::from([
                    // ((reference,compute), success_rate)
                    // TODO
                    (SupportedNrOperands::Two, 97.5.into()),
                    (SupportedNrOperands::Four, 97.0.into()),
                    (SupportedNrOperands::Eight, 28.0.into()),
                    (SupportedNrOperands::Sixteen, 10.0.into()),
                ]),
            LogicOp::NOR => HashMap::from([
                    // ((reference,compute), success_rate)
                    // TODO
                    (SupportedNrOperands::Two, 97.5.into()),
                    (SupportedNrOperands::Four, 97.0.into()),
                    (SupportedNrOperands::Eight, 28.0.into()),
                    (SupportedNrOperands::Sixteen, 10.0.into()),
                ]),
            }
    }
}

/// Support operands numbers for AND/OR/NOT operations
#[derive(Debug, Clone, Copy, EnumIter, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)] // You can change the representation (e.g., u8, u16, etc.)
pub enum SupportedNrOperands {
    /// One operand only supported for `NOT`
    One = 1,
    Two = 2,
    Four = 4,
    Eight = 8,
    Sixteen = 16,
    /// Only performed for `NOT`
    Thirtytwo = 32
}

impl TryFrom<u8> for SupportedNrOperands {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(SupportedNrOperands::One),
            2 => Ok(SupportedNrOperands::Two),
            4 => Ok(SupportedNrOperands::Four),
            8 => Ok(SupportedNrOperands::Eight),
            16 => Ok(SupportedNrOperands::Sixteen),
            32 => Ok(SupportedNrOperands::Thirtytwo),
            _ => Err(()),
        }
    }
}

impl TryFrom<usize> for SupportedNrOperands {
    type Error = ();

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        Self::try_from(value as u8)
    }
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
    // - NOTE: currenlty N:2N activation pattern is <u>not</u> supported
}

// TODO
#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_sra() {
        println!("{:?}", ARCHITECTURE.sra_degree_to_rowaddress_combinations.get(&SupportedNrOperands::try_from(8 as u8).unwrap()).unwrap().first());
    }
}
