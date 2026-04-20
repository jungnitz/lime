//!
//! - [`Compiler`] = performs actual compilation
//!     - [`CompilationState`] = stores states encountered during compilation (eg which values reside in which rows, rows containing live values, ..)
//!         - also see [`RowState`]
//! - [`SchedulingPrio`] = used to prioritize/order instruction for Instruction Scheduling
//!
//! - [`Compiler::compile()`] = main function - compiles given logic network for the given [`architecture`] into a [`program`] using some [`optimization`]

use crate::fc_dram::architecture::{Instruction, ROWS_PER_SUBARRAY};

use super::{
    architecture::{LogicOp, NeighboringSubarrayRelPosition, SubarrayId, SupportedNrOperands, ARCHITECTURE, NR_SUBARRAYS, ROW_ID_BITMASK}, CompilerSettings, Program, RowAddress
};
use eggmock::{Aoig, Id, NetworkWithBackwardEdges, Node, Signal};
use log::debug;
use priority_queue::PriorityQueue;
use strum::IntoEnumIterator;
use std::{cmp::Ordering, collections::HashMap, ffi::CStr, fmt::Debug, fs::{self, File}, io::Write, path::Path, vec};

use serde::{Serialize, Deserialize};

/// Provides [`Compiler::compile()`] to compile a logic network into a [`Program`]
pub struct Compiler {
    /// compiler-options set by user
    settings: CompilerSettings,
    /// Stores the state of all rows at each compilation step
    comp_state: CompilationState,
    /// For each nr of operands this field store the rowaddress-combination to issue to activate
    /// the desired nr of rows (the choice is made best on success-rate and maximizing the nr of rows which potentially can't be used for storage
    /// since they would activate rows where values could reside in)
    /// - This is a Design Decision taken: compute rows are rows reserved for performing computations, all other rows are usable as "Register"
    compute_row_activations: HashMap<(SupportedNrOperands, NeighboringSubarrayRelPosition), (RowAddress,RowAddress)>,
    /// Stores all subarrays in which the signal has to be available
    signal_to_subarrayids: HashMap<Signal, Vec<SubarrayId>>,
    /// see [`Self::get_all_noninverted_src_signals`]. First `Vec<Signal>`=noninverted src signals, 2nd `Vec<Signal>`=inverted src signals
    computed_noninverted_scr_signals: HashMap<Signal, (Vec<Signal>,Vec<Signal>)>,
}

/// Serializable struct for storing chosen compute rows in a json file
#[derive(Serialize,Deserialize)]
struct ComputeRowRecord {
    operands: SupportedNrOperands,
    position: NeighboringSubarrayRelPosition,
    rows: (RowAddress, RowAddress),
}

impl Compiler {
    /// Constants are repeated to fill complete row
    const CONSTANTS: [usize; 2] = [0, 1];

    pub fn new(settings: CompilerSettings) -> Self {
        Compiler{
           settings,
           comp_state: CompilationState::new( HashMap::new() ),
           compute_row_activations: HashMap::new(),
           signal_to_subarrayids: HashMap::new(),
           computed_noninverted_scr_signals: HashMap::new(),
        }
    }

    /// Compiles given `network` into a FCDRAM-[`Program`] that can be run on given `architecture`
    ///
    /// General Procedure of compilation
    /// 1) Map Logical-Ops to FCDRAM-Primitives (operating on virtual rows)
    /// 2) Map virtual rows to actual physical rows (spilling/moving rows if necessary using `RowClone`)
    ///     - similarly to Register Allocation
    ///
    /// - [ ]  TODO: increase success-rate using input replication ? at which point to add input replication?
    ///
    /// - [ ] TODO: output in which rows
    ///     - 1) data is expected to be placed before program runs
    ///     - 2) outputs can be found after the program has run
    pub fn compile(
        &mut self,
        network: &impl NetworkWithBackwardEdges<Node = Aoig>,
    ) -> Program {
        let mut program = Program::new(vec!());

        // debug!("Compiling {:?}", network);
        // 0. Prepare compilation:
        // - select safe-space rows
        // - place inputs&constants into DRAM module (and store where inputs have been placed in `program`)
        // - initialize candidates (with which to start execution)
        self.init_comp_state(network, &mut program);

        // println!("{:?}", network.outputs().collect::<Vec<Signal>>());
        // debug!("Nodes in network:");
        // for node in network.iter() {
        //     debug!("{:?},", node);
        // }

        // 1. Actual compilation
        while let Some((next_candidate, _)) = self.comp_state.candidates.pop() {
                let executed_instructions = &mut self.execute_next_instruction(&next_candidate, network);
                program.instructions.append(executed_instructions);

                // update new candidates (`next_candidate` is now available)
                let new_candidates = self.get_new_candidates(network, next_candidate.0, next_candidate.1);
                debug!("New candidates: {:?}", new_candidates);

                self.comp_state.candidates.extend(new_candidates);
        }

        debug!("Instructions: {:?}", program.instructions);
        // optimize(&mut program);
        debug!("{:?}", self.comp_state.value_states);

        // store output operand location so user can retrieve them after running the program
        let outputs = network.outputs();
        // TODO: doesn't work yet
        program.output_row_operands_placement = outputs.flat_map(|out| {
            let subarrays = self.signal_to_subarrayids.get(&out).unwrap();
            let mut placements = vec!();
            for subarray in subarrays {
                let row_address = self.comp_state.value_states.get(&(out,*subarray)).expect("ERROR: one of the outputs hasn't been computed yet...");
                placements.push((out, *row_address));
            }
            placements
        }).collect();
        program
    }

    /// Rather than making sure rows in which live values reside remain untouched, this approach chooses to select fixed RowAddress combinations for all support numbers of operands
    /// - this function sets [`Compiler::compute_row_activations`] to use as compute rows
    /// - NOTE: this choice is expected to be applicable to row activations in all subarrays since the SRA work equivalently between subarrays
    ///     - ASSUMPTION: there are no architectural differences btw subarrays
    ///
    /// # Limitations
    ///
    /// There are several drawbacks of choosing fixed compute rows:
    /// 1. *LogicOp* is not taken into consideration: different compute rows might (in theory) perform better for specific LogicOps (see [`LogicOp::get_success_rate_by_row_distance()`] which returns different SuccessRates based on the corresponding LogicOp)
    /// 2. Compute Rows might perform better for the next subarray (+1) than for the previous (-1) subarray (choice of subarray determines which SenseAmps are used and hence the distance btw rows and SenseAmps)
    ///
    /// This choice aims to finding a good compromise btw those limitations.
    /// TODO: NEXT
    fn choose_compute_rows(&mut self) {
        for nr_operands in SupportedNrOperands::iter() {
            for sense_amp_position in NeighboringSubarrayRelPosition::iter() {

                let possible_row_combis = ARCHITECTURE.sra_degree_to_rowaddress_combinations.get(&nr_operands).expect("Given Architecture doesn't support SRA of {nr_operands} operands");
                let best_row_combi = possible_row_combis.iter().fold(possible_row_combis[0], |best_row_combi, next_row_combi| {
                    // compare row-combis based on avg success-rate and return the better one of them

                    let avg_distance_next_row_combi: u64 = {
                        let activated_rows = ARCHITECTURE.precomputed_simultaneous_row_activations.get(next_row_combi).unwrap();
                        activated_rows.iter().map(|&row| {
                            // move subarray to 1st subarray (instead of 0th, which is at the edge and hence has no sense-amps above)
                            let subarray1_id = ((ROW_ID_BITMASK << 1) | 1) ^ ROW_ID_BITMASK;
                            let row =  RowAddress(subarray1_id | row.0); // makes sure that `get_distance_of_row_to_sense_amps` doesn't panic since SRA returns subarray=0 by default (which is an edge subarray)
                            (ARCHITECTURE.get_distance_of_row_to_sense_amps)(row, sense_amp_position) as u64
                        }).sum()
                    };
                    let avg_distance_best_row_combi: u64 = {
                        // move subarray to 1st subarray (instead of 0th, which is at the edge and hence has no sense-amps above)
                        let activated_rows = ARCHITECTURE.precomputed_simultaneous_row_activations.get(&best_row_combi).unwrap();
                        activated_rows.iter().map(|&row| {
                            let subarray1_id = ((ROW_ID_BITMASK << 1) | 1) ^ ROW_ID_BITMASK;
                            let row =  RowAddress(subarray1_id | row.0); // makes sure that `get_distance_of_row_to_sense_amps` doesn't panic since SRA returns subarray=0 by default (which is an edge subarray)
                            // println!("{:b}", row.0);
                            (ARCHITECTURE.get_distance_of_row_to_sense_amps)(row, sense_amp_position) as u64
                        }).sum()
                    };

                    if avg_distance_next_row_combi > avg_distance_best_row_combi {
                        *next_row_combi
                    } else {
                        best_row_combi
                    }
                });

                self.compute_row_activations.insert((nr_operands, sense_amp_position), best_row_combi);
            }
        }
    }

    /// Places (commonly used) constants in safe-space rows
    /// - ! all safe-space rows are assumed to be empty when placing constants (constans are the first things to be placed into safe-space rows)
    /// - currently placed constants: all 0s and all 1s (for [`Compiler::init_reference_subarray`]
    /// - TODO: store placement of constants in `program`
    fn place_constants(&mut self, program: &mut Program) {
        // place constants in EVERY subarray
        for subarray in 0..NR_SUBARRAYS {
            for constant in Self::CONSTANTS {
                let next_free_row = self.comp_state.free_rows_per_subarray.get_mut(&SubarrayId(subarray)).and_then(|v| v.pop()).expect("No free rows in subarray {subarray} :(");
                self.comp_state.constant_values.insert(constant, next_free_row.local_rowaddress_to_subarray_id(SubarrayId(0)));
                self.comp_state.dram_state.insert(next_free_row, RowState { is_compute_row: false, live_value: None, constant: Some(constant)} );
                program.constants_row_placement.insert(constant, next_free_row);
            }
        }
    }

    /// Place inputs onto appropriate rows, storing the decided placement into `program.input_row_operands_placement`
    /// - NOTE: constants are expected to be placed before the inputs
    /// TODO: algo which looks ahead which input-row-placement might be optimal (->reduce nr of move-ops to move intermediate results around & keep inputs close to sense-amps
    /// TODO: parititon logic network into subgraphs s.t. subgraphs can be mapped onto subarrays reducing nr of needed moves
    fn place_inputs(&mut self, network: &impl NetworkWithBackwardEdges<Node = Aoig>, program: &mut Program) {

        for input in network.leafs().collect::<Vec<Id>>() {
            // check whether the signal is required in inverted or noninverted form and place it accordingly in all subarrays where it is needed
            let original_signal = Signal::new(input, false);
            let inverted_signal = Signal::new(input, true);

            if let Some(original_input_locations) = self.signal_to_subarrayids.get(&original_signal) {
                for subarray in original_input_locations {
                    let next_free_row = self.comp_state.free_rows_per_subarray.get_mut(subarray).and_then(|v| v.pop()).expect("OOM: No more free rows in subarray {subarray} for placing inputs");
                    self.comp_state.value_states.insert((original_signal, *subarray), next_free_row);

                    program.input_row_operands_placement.entry(original_signal).or_default().push(next_free_row);
                }
            }

            if let Some(inverted_input_locations) = self.signal_to_subarrayids.get(&inverted_signal) {
                for subarray in inverted_input_locations {
                    let next_free_row = self.comp_state.free_rows_per_subarray.get_mut(subarray).and_then(|v| v.pop()).expect("OOM: No more free rows in subarray {subarray} for placing inputs");
                    self.comp_state.value_states.insert((inverted_signal, *subarray), next_free_row);

                    program.input_row_operands_placement.entry(inverted_signal).or_default().push(next_free_row);
                }
            }
        }
    }

    /// Initialize candidates with all nodes that are computable
    /// NOTE: initially all nodes whose src-operands are primary inputs only are marked as candidates in <u>all</u> subarrays (since inputs are expected to be placed in all those subarrays by the user)
    fn init_candidates(&mut self, network: &impl NetworkWithBackwardEdges<Node = Aoig>) {
        let inputs: Vec<Id> = network.leafs().collect();

        // init candidates with all nodes having only inputs as src-operands
        for &input in inputs.as_slice() {
            // every output has a prio determined eg by how many src-operands it uses last (->to minimize nr of live values in rows)
            let mut outputs_with_prio: PriorityQueue<(Signal, SubarrayId), SchedulingPrio> = network.node_outputs(input)
                .filter(|output| network.node(*output).inputs().iter().all(|other_input| inputs.contains(&other_input.node_id()) )) // only those nodes are candidates, whose src-operands are ALL inputs (->only primary inputs are directly available)
                .flat_map( |output| {
                    let output_signal = Signal::new(output, false);
                    let mut output_candidates = vec!();

                    // for every subarray in which the signal is needed
                    for subarray in self.signal_to_subarrayids.get(&output_signal).expect("Signal is not mapped to a subarray yet??") {
                        output_candidates.push(((output_signal,*subarray), self.compute_scheduling_prio_for_node(output_signal, *subarray, network)));
                    }

                    // if negation is also needed
                    let inverted_output_signal = Signal::new(output, true);
                    if self.signal_to_subarrayids.contains_key(&inverted_output_signal) {
                        // for every subarray in which the signal is needed
                        for subarray in self.signal_to_subarrayids.get(&inverted_output_signal).expect("Signal is not mapped to a subarray yet??") {
                            output_candidates.push(((inverted_output_signal,*subarray), self.compute_scheduling_prio_for_node(inverted_output_signal, *subarray, network)));
                        }
                    }

                    output_candidates
                })
                .collect();

            self.comp_state.candidates.append(&mut outputs_with_prio);
            debug!("{:?} has the following outputs: {:?}", input, network.node_outputs(input).collect::<Vec<Id>>());
        }
    }

    /// Returns list of candidates that can be computed once `computed_node` is available
    fn get_new_candidates(&mut self, network: &impl NetworkWithBackwardEdges<Node = Aoig>, computed_node: Signal, subarray: SubarrayId) -> PriorityQueue<(Signal, SubarrayId), SchedulingPrio> {
        debug!("Candidates: {:?}", self.comp_state.candidates);
        debug!("DRAM state: {:?}", self.comp_state.value_states);
        network.node_outputs(computed_node.node_id())
            // filter for new nodes that have all their input-operands available now in the same subarray (->only inputs of computed nodes could have changed to candidate-state, other nodes remain uneffected)
            .filter({|out| network.node(*out).inputs().iter()
                .all( |input| {
                    debug!("OUTPUT: {out:?} with input {input:?}");
                    debug!("Out: {:?}, In: {:?}", out, input);
                    self.comp_state.value_states.contains_key(&(*input, subarray))
                })
            })
            .flat_map(|id| {
                let noninverted_signal = Signal::new(id, false);
                let mut new_candidates = vec!(((noninverted_signal, subarray), self.compute_scheduling_prio_for_node(noninverted_signal, subarray, network))); // noninverted version needs to be computed anyway
                let inverted_signal = Signal::new(id, true);
                // if needed also schedule `inverted_signal` for computation
                if self.signal_to_subarrayids.contains_key(&inverted_signal) {
                    new_candidates.push(((inverted_signal, subarray.get_partner_subarray()), self.compute_scheduling_prio_for_node(noninverted_signal, subarray, network)));
                }
                new_candidates.into_iter().collect::<PriorityQueue<(Signal, SubarrayId), SchedulingPrio>>()
            }) // TODO: check if inverted signal is required as well!
            .collect()
    }

    /// Initialize compilation state:
    /// - choose compute rows (by setting [`Self::compute_row_activations`]
    /// - decide in which rows to place constants
    /// - assign subarray-ids to each NodeId
    /// - return code to place input operands in `program`
    fn init_comp_state(&mut self, network: &impl NetworkWithBackwardEdges<Node = Aoig>, program: &mut Program) {
        let config_file = unsafe { CStr::from_ptr(self.settings.config_file) }.to_str().unwrap();
        let config = Path::new(config_file);

        // 0.1 Allocate compute rows: rows reserved for performing computations, all other rows are usable as "Register"
        if config.is_file() {
            // if config-file has been provided: get compute rows from that config file rather than recomputing them

            let contents = fs::read_to_string(config).unwrap();
            // Read file contents

            // Parse JSON into Vec<RowActivationRecord>
            let records: Vec<ComputeRowRecord> =
                serde_json::from_str(&contents).unwrap_or_else(|_| panic!("Failed to parse JSON for file {:?}", config.to_str()));

            // Convert into the HashMap structure
            self.compute_row_activations = records
                .into_iter()
                .map(|rec| {
                    (
                        (rec.operands, rec.position),
                        (rec.rows.0, rec.rows.1),
                    )
                })
            .collect();

            // This is the result for SKHYNIX DRAM (comment out if to save dev-time generating json-file):
            // self.compute_row_activations = HashMap::from([
            //     ((SupportedNrOperands::One, NeighboringSubarrayRelPosition::Above), (RowAddress(8), RowAddress(8))),
            //     ((SupportedNrOperands::One, NeighboringSubarrayRelPosition::Below), (RowAddress(303), RowAddress(303))),
            //     ((SupportedNrOperands::Two, NeighboringSubarrayRelPosition::Above), (RowAddress(15), RowAddress(79))),
            //     ((SupportedNrOperands::Two, NeighboringSubarrayRelPosition::Below), (RowAddress(293), RowAddress(357))),
            //     ((SupportedNrOperands::Four, NeighboringSubarrayRelPosition::Above), (RowAddress(60), RowAddress(42))),
            //     ((SupportedNrOperands::Four, NeighboringSubarrayRelPosition::Below), (RowAddress(472), RowAddress(412))),
            //     ((SupportedNrOperands::Eight, NeighboringSubarrayRelPosition::Above), (RowAddress(42), RowAddress(15))),
            //     ((SupportedNrOperands::Eight, NeighboringSubarrayRelPosition::Below), (RowAddress(203), RowAddress(283))),
            //     ((SupportedNrOperands::Sixteen, NeighboringSubarrayRelPosition::Above), (RowAddress(32), RowAddress(83))),
            //     ((SupportedNrOperands::Sixteen, NeighboringSubarrayRelPosition::Below), (RowAddress(470), RowAddress(252))),
            //     ((SupportedNrOperands::Thirtytwo, NeighboringSubarrayRelPosition::Above), (RowAddress(307), RowAddress(28))),
            //     ((SupportedNrOperands::Thirtytwo, NeighboringSubarrayRelPosition::Below), (RowAddress(149), RowAddress(318))),
            // ]);


        } else {
            self.choose_compute_rows(); // choose which rows will serve as compute rows (those are stored in `self.compute_row_activations`

            // write chosen compute rows to config-file
            let records: Vec<ComputeRowRecord> = self.compute_row_activations
                .iter()
                .map(|((operands, position), (row1, row2))| ComputeRowRecord {
                    operands: *operands,
                    position: *position,
                    rows: (*row1, *row2),
                })
            .collect();

            // Store compute-row choice in json file
            let json_output = serde_json::to_string_pretty(&records).unwrap();
            // Write to file
            let output_path = Path::new("fcdram_hksynx_compute_rows.json"); // NOTE: the chosen compute rows are specific to HKSYNX DRAM for now
            let mut file = File::create(output_path).expect("Failed to create output file");
            file.write_all(json_output.as_bytes()).expect("Failed to write JSON to file");
            println!("Stored chosen compute-rows into {:?}. Pass this file via `.config_file` to safe considerate compilation time for choosing the compute rows", output_path.to_str());
        }

        // 0.2 Save free rows
        // At the start all rows, except for the compute rows, are free rows
        let compute_rows = self.compute_row_activations.values().fold(vec!(), |all_compute_rows, next_compute_row_combi| {
            let new_compute_rows = ARCHITECTURE.precomputed_simultaneous_row_activations.get(next_compute_row_combi).expect("Compute row can't be activated??");
            all_compute_rows.iter().chain(new_compute_rows).cloned().collect()
        });
        let mut free_rows = (0..ROWS_PER_SUBARRAY).map(RowAddress::from).collect::<Vec<RowAddress>>();
        free_rows.retain(|r| {!compute_rows.contains(r)});
        for subarray in 0..NR_SUBARRAYS {
            let free_rows_in_subarray = free_rows.iter().map(|row| row.local_rowaddress_to_subarray_id(SubarrayId(subarray))).collect(); // transform local row address to row addresses in corresponding `subarray`
            self.comp_state.free_rows_per_subarray.entry(SubarrayId(subarray)).insert_entry(free_rows_in_subarray);
        }

        // 0.3 Group operands by subarray (ensure all operands are placed in the right subarray)
        self.assign_signals_to_subarrays(network); // sets `self.signal_to_subarrayids`

        // NEXT: 0.4 Place all constants and inputs and mark the inputs as being live
        self.place_constants(program); // constants are placed in <u>each</u> subarray
        debug!("Placed constants: {:?}", self.comp_state.constant_values);

        self.place_inputs(network, program); // place input-operands into rows
        debug!("Placed inputs {:?} in {:?}", network.leafs().collect::<Vec<Id>>(), self.comp_state.value_states);

        // 0.5 Setup: store all network-nodes yet to be compiled
        self.init_candidates(network);
        debug!("Initialized candidates {:?}", self.comp_state.candidates);
    }

    /// Assigns signals to subarrays and through this determines placement of those signal in the DRAM module
    /// - sets [`Self::signal_to_subarrayids`]
    ///
    /// # Assumptions
    ///
    /// - assumes `network` is acyclic !
    ///
    /// # TODO
    ///
    /// - make sure nr of signals placed in a subarray is <= nr of available rows (without compute rows)
    /// - think about merging subarray assignment (s.t. several outputs end up in same subarray, so that inputs can be reused among them)
    fn assign_signals_to_subarrays(&mut self, network: &impl NetworkWithBackwardEdges<Node = Aoig>) {
        // 1. determine all signals which go into outputs without being negated (and hence can be stored in same subarray)
        //  - also store signals which do need to be negated (and process them in the next step)
        //  - TODO: continue graph traversal with src-operands of the outputs (until primary inputs are reached)
        let mut subarray_id = 1; // start with 1 since edge subarrays cant be used as compute subarrays
        for output in network.outputs() {
            self.signal_to_subarrayids.insert(output, vec!(SubarrayId(subarray_id))); // determine (virtual) subarray in which output will reside - assignment might change in a later stage by merging several outputs into same subarray if there is enough space
            let neighboring_subarray = SubarrayId(subarray_id).get_partner_subarray();
            let (actual_subarray , neighboring_subarray) = {

                // if output is inverted, then the non-inverted value resides in the partner subarray
                if output.is_inverted() {
                    self.signal_to_subarrayids.insert(Signal::new(output.node_id(), false), vec!(neighboring_subarray)); // determine (virtual) subarray in which output will reside
                    (neighboring_subarray, SubarrayId(subarray_id))
                } else {
                    (SubarrayId(subarray_id), neighboring_subarray)
                }
            };

            let (noninverted_src_signals, inverted_src_signals) = self.get_all_noninverted_and_inverted_src_signals(output, network);

            // println!("Noninverted src signals: {:?}", noninverted_src_signals.clone());
            // all directly (might in theory) reside in the same subarray as `output` (since no NOTS are inbtw which locate them to a neighboring subarray)
            for connected_signal in noninverted_src_signals {
                self.signal_to_subarrayids.entry(connected_signal).or_default().push(actual_subarray); // determine (virtual) subarray in which output will reside
            }

            // Place direct inputs that ARE inverted in same subarray (while their non-inverted version will end up in the neighboring subarray)
            for inverted_signal in inverted_src_signals.as_slice() {
                self.signal_to_subarrayids.entry(*inverted_signal).or_default().push(actual_subarray); // determine (virtual) subarray in which output will reside
            }
            // place inverted signals in neighboring subarray, 2x inverted signals in same subarray, 3x inverted signals in neighboring subarray etc...
            // TODO: same thing as before: place non-inverted version of inverted signals in opposite subarray !
            // !!! doesnt support >=2 NOTs on one path yet !!!
            let mut unvisited_signals_in_same_subarray: Vec<Signal> = vec!(); // inverting even nr of times leads to signals being placed in same subarray
            let mut unvisited_signals_in_neighboring_subarray: Vec<Signal> = inverted_src_signals.iter() // =those signals that are negated an odd nr of times
                .filter(|signal| !network.node(signal.node_id()).is_leaf() ) // leaves don't need to be placed in neighboring subarray since inputs are placed by user
                .map(|signal| Signal::new(signal.node_id(), false)).collect(); // before negation the signals are in the neighboring subarray
            while !unvisited_signals_in_same_subarray.is_empty() || !unvisited_signals_in_neighboring_subarray.is_empty() {
                // println!("Same subarray: {:?}", unvisited_signals_in_same_subarray);
                // println!("Neighboring subarray: {:?}", unvisited_signals_in_neighboring_subarray);
                if let Some(signal_neighboring_subarray) = unvisited_signals_in_neighboring_subarray.pop() {

                    debug!("Neighboring: {signal_neighboring_subarray:?}");
                    self.signal_to_subarrayids.entry(signal_neighboring_subarray).or_default().push(neighboring_subarray);
                    // these are placed in the Above subarray (arbitrary decision, epxloring whether this makes a difference might be explored in future)
                    // NOTE: signals that are inverted an even nr of times are placed in the same subarray as the output
                    let (signals_neighboring_subarray,  signals_inverted_even_nr_times) = self.get_all_noninverted_and_inverted_src_signals(signal_neighboring_subarray, network);
                    for signal_inverted_odd_nr_times in signals_neighboring_subarray {
                        debug!("ODD: {signal_inverted_odd_nr_times:?} placed in subarray {neighboring_subarray}");
                        self.signal_to_subarrayids.entry(signal_inverted_odd_nr_times).or_default().push(neighboring_subarray);
                    }

                    // signals which are inverted again require the non-inverted version to be in the other subarray
                    let mut signals_to_invert_once_more: Vec<Signal> = signals_inverted_even_nr_times.into_iter().filter(|signal| {
                        if  network.node(signal.node_id()).is_leaf() {
                            // is input signal
                            self.signal_to_subarrayids.entry(*signal).or_default().push(neighboring_subarray);
                            false
                        } else { true }
                    }).collect(); // inputs are placed by user (also inverted ones)
                    for even_times_inverted_signals in signals_to_invert_once_more.as_slice() {
                        let signal = Signal::new(even_times_inverted_signals.node_id(), false);
                        self.signal_to_subarrayids.entry(signal).or_default().push(actual_subarray);
                    }
                    unvisited_signals_in_same_subarray.append(&mut signals_to_invert_once_more);
                }

                if let Some(signal_same_subarray) = unvisited_signals_in_same_subarray.pop() {

                    debug!("Same: {signal_same_subarray:?}");
                    self.signal_to_subarrayids.entry(signal_same_subarray).or_default().push(actual_subarray);
                    // signals inverted even nr of times are placed in the same subarray as the `output` Signal
                    // NOTE: signals that are inverted an odd nr of times are placed in the neighboring subarray of the output
                    let (signals_same_subarray_of_output,  signals_inverted_even_nr_times) = self.get_all_noninverted_and_inverted_src_signals(signal_same_subarray, network);
                    for signal in signals_same_subarray_of_output {
                        self.signal_to_subarrayids.entry(signal).or_default().push(actual_subarray);
                    }

                    // signals which are inverted again require the non-inverted version to be in the other subarray
                    let mut signals_to_invert_once_more: Vec<Signal> = signals_inverted_even_nr_times.into_iter().filter(|signal| {
                        if  network.node(signal.node_id()).is_leaf() {
                            // is input signal
                            self.signal_to_subarrayids.entry(*signal).or_default().push(neighboring_subarray);
                            false
                        } else { true }
                    }).collect(); // inputs are placed by user (also inverted ones)
                    for even_times_inverted_signals in signals_to_invert_once_more.as_slice() {
                        let signal = Signal::new(even_times_inverted_signals.node_id(), false);
                        self.signal_to_subarrayids.entry(signal).or_default().push(neighboring_subarray);
                    }
                    unvisited_signals_in_neighboring_subarray.append(&mut signals_to_invert_once_more);
                }
            }

            // for the beginning place all outputs in different subarrays. A 2nd pass may optimize/merge subarrays later on
            subarray_id += 2; // maybe +=2 to account for negated operands being stored in neighboring subarray? (TODO: test with some example networks)
        }


        debug!("Signals to subarrayids: {:?}", self.signal_to_subarrayids);
    }

    /// Returns all src signals which are not inverted. These are exactly those signals that can be placed in the same subarray as.
    ///
    /// # Returns
    ///
    /// Tuple of
    /// 1. Vector of src Signals that are **not** inverted
    /// 2. Vector of src Signals that are indeed inverted (need to be processed further, only first inverted signal is returned for a subtree)
    fn get_all_noninverted_and_inverted_src_signals(&mut self, signal: Signal, network: &impl NetworkWithBackwardEdges<Node = Aoig>) -> (Vec<Signal>, Vec<Signal>) {
        let signal_node = network.node(signal.node_id());

        let mut noninverted_src_signals = vec!();
        let mut inverted_src_signals = vec!();
        let mut stack_unvisited_noninverted_src_operands = Vec::from(signal_node.inputs());

        while let Some(src_operand) = stack_unvisited_noninverted_src_operands.pop() {
            if src_operand.is_inverted() {
                inverted_src_signals.push(src_operand); // store subarray to which this input has to be placed to as a neighbor, further processing elsewhere
            } else {
                noninverted_src_signals.push(src_operand);
                let src_operand_node = network.node(src_operand.node_id());
                stack_unvisited_noninverted_src_operands.append(&mut Vec::from(src_operand_node.inputs()));
            }
        }

        self.computed_noninverted_scr_signals.insert(signal, (inverted_src_signals.clone(), noninverted_src_signals.clone())); // to save (possible) recomputation next time
        (noninverted_src_signals, inverted_src_signals)
    }

    /// Returns instructions to initialize all given `ref_rows` in reference-subarray for corresponding logic-op
    /// - NOTE: [1] doesn't describe how the 0s/1s get into the reference subarray. We use `RowCloneFPM` ([4])) to copy the constant 0s/1s from the reserved safe-space row into the corresponding reference subarray row
    fn init_reference_subarray(&self, mut ref_rows: Vec<RowAddress>, logic_op: LogicOp) -> Vec<Instruction> {
        match logic_op {
            LogicOp::AND => {
                let frac_row = ref_rows.pop().expect("Min 1 row has to be passed for initializing ref subarray"); // TODO: include success-rate considerations to choose best row to use for storing `V_{DD}/2`
                let row_address_1 = self.comp_state.constant_values.get(&1).expect("Constants are expected to be placed in every subarray beforehand")
                    .local_rowaddress_to_subarray_id(frac_row.get_subarray_id()); // row address where all 1s (V_DD) are to bestored
                let mut instructions = vec!();
                for _ in 0..self.settings.repetition_fracops {
                    instructions.push(Instruction::FracOp(frac_row));
                }
                for other_row in ref_rows {
                    instructions.push(Instruction::RowCloneFPM(row_address_1, other_row, String::from("Init ref-subarray with 1s")));
                }
                instructions
            },
            LogicOp::OR => {
                let frac_row = ref_rows.pop().expect("Min 1 row has to be passed for initializing ref subarray"); // TODO: include success-rate considerations to choose best row to use for storing `V_{DD}/2`
                let row_address_0 = self.comp_state.constant_values.get(&0).expect("Constants are expected to be placed in every subarray beforehand")
                    .local_rowaddress_to_subarray_id(frac_row.get_subarray_id()); // row address where all 0s (GND) are to be stored
                let mut instructions = vec!();
                for _ in 0..self.settings.repetition_fracops {
                    instructions.push(Instruction::FracOp(frac_row));
                }
                for other_row in ref_rows {
                    instructions.push(Instruction::RowCloneFPM(row_address_0, other_row, String::from("Init ref-subarray with 0s")));
                }
                instructions
            },
            LogicOp::NOT => vec!(),
            _ => panic!("{logic_op:?} not supported yet"),
        }
    }

    /// Returns instructions to be executed for performing `NOT` on `src_row` into `dst_row` and updates the `comp_state` holding the negated value
    /// - NOTE: currenlty only single-operand NOTs are supported bc
    ///     1) more operands lead to (slightly) worse results (see Figure10 in [1])
    ///     2) since there are separate compute rows using multiple dst rows doesn't make sense (the values need to be copied out of the dst-rows anyway into non-compute rows)
    fn execute_not(&mut self, signal_to_invert: &Signal, dst_array: SubarrayId) -> Vec<Instruction> {
        let mut instructions = vec!();
        let row_combi = self.compute_row_activations.get(&(SupportedNrOperands::One, NeighboringSubarrayRelPosition::Above)).unwrap();
        let src_array = dst_array.get_partner_subarray();

        // 1. Copy non-inverted operand into src-row
        let src_row = row_combi.0.local_rowaddress_to_subarray_id(src_array);
        let unnegated_signal = Signal::new(signal_to_invert.node_id(), false);
        let src_location = self.comp_state.value_states.get(&(unnegated_signal, src_array)).unwrap_or_else(|| panic!("Src operand {src_row} is not live in subarray {src_array} (see {signal_to_invert:?})??"));

        instructions.push(self.execute_intrasubarray_rowclone(*src_location, src_row, String::from("Move into compute row")));

        // 2. Execute NOT
        let dst_row = row_combi.1.local_rowaddress_to_subarray_id(dst_array);
        instructions.push(Instruction::ApaNOT(src_row, dst_row));

        // 3. Copy negated value out of compute rows
        let free_row = self.comp_state.free_rows_per_subarray.get_mut(&dst_array).and_then(|v| v.pop()).unwrap_or_else(|| panic!("OOM: No free rows in subarray {dst_array}"));
        instructions.push(self.execute_intrasubarray_rowclone(dst_row, free_row, String::from("Move into free row")));
        self.comp_state.value_states.insert((*signal_to_invert, dst_array), free_row);

        instructions
    }

    /// Returns the instructions needed to perform `language_op` placing the result in a free row in the `compute_subarray`
    /// - TODO: also store negated signal (in reference subarray) if it's needed later on?
    fn execute_and_or(&mut self, node_id: Id, compute_subarray: SubarrayId, network: &impl NetworkWithBackwardEdges<Node = Aoig>) -> Vec<Instruction> {
        let mut instructions = vec!();

        let (reference_subarray, rel_position_sense_amps_to_compute_subarray) = if compute_subarray.0 % 2 == 0 { // currently the following arrays are compute&reference subarrays of each other: 0&1,2&3,4&5,...
            (compute_subarray.0 + 1, NeighboringSubarrayRelPosition::Below)
        } else {
            (compute_subarray.0 - 1, NeighboringSubarrayRelPosition::Above)
        };


        let nr_src_operands = SupportedNrOperands::try_from(network.node(node_id).inputs().len()).unwrap();
        let compute_row_combi= self.compute_row_activations.get(&(nr_src_operands, rel_position_sense_amps_to_compute_subarray)).expect("Nr input operands is {}");
        let compute_rows = ARCHITECTURE.precomputed_simultaneous_row_activations.get(compute_row_combi).unwrap();

        // 0. Move all src_operands into compute rows
        // TODO: map compute rows to right subarray
        for (&compute_row, &src_signal) in compute_rows.iter().zip(network.node(node_id).inputs()) {
            let compute_row = compute_row.local_rowaddress_to_subarray_id(compute_subarray);
            let &src_row = self.comp_state.value_states.get(&(src_signal, compute_subarray)).unwrap_or_else(|| panic!("Src signal {src_signal:?} is not present in compute subarray {compute_subarray} ???"));
            instructions.push(self.execute_intrasubarray_rowclone(src_row, compute_row, String::from("Move into compute row")));
        }


        let language_op = network.node(node_id);
        let logic_op = match language_op {
            // REMINDER: operand-nr is extracted by looking at nr of children beforehand
            Aoig::And(_) | Aoig::And4(_) | Aoig::And8(_)| Aoig::And16(_) => LogicOp::AND,
            Aoig::Or(_) | Aoig::Or4(_) | Aoig::Or8(_) | Aoig::Or16(_) => LogicOp::OR,
            _ => panic!("candidate is expected to be a logic op"),
        };

        // 1. Initialize reference subarray
        let ref_rows = compute_rows.iter().map(|c| c.local_rowaddress_to_subarray_id(SubarrayId(reference_subarray))).collect();
        instructions.append(&mut self.init_reference_subarray(ref_rows, logic_op));

        // 2. Execute actual computation
        instructions.push(Instruction::ApaAndOr(compute_row_combi.0.local_rowaddress_to_subarray_id(compute_subarray), compute_row_combi.1.local_rowaddress_to_subarray_id(SubarrayId(reference_subarray))));

        // 3. Move result into non-compute row
        // TODO: add instruction to move value into free-row !!
        let free_row = self.comp_state.free_rows_per_subarray.get_mut(&compute_subarray).and_then(|v| v.pop()).unwrap_or_else(|| panic!("No more free rows in subarray {compute_subarray}"));
        instructions.push(self.execute_intrasubarray_rowclone(compute_rows[0], free_row, String::from("Move into free row")));
        self.comp_state.value_states.insert((Signal::new(node_id, false), compute_subarray), free_row); // TODO: for inverted signals not in result_subarray, right?

        instructions
    }

    fn execute_intrasubarray_rowclone(&self, src_row: RowAddress, dst_row: RowAddress, comment: String) -> Instruction {
        Instruction::RowCloneFPM(src_row, dst_row, comment)
    }

    /// Returns Instructions to execute given `next_candidate` (which is a signal which needs to reside in a specific subarray after performing the execution)
    /// - [ ] make sure that operation to be executed on those rows won't simultaneously activate other rows holding valid data which will be used by future operations
    fn execute_next_instruction(&mut self, next_candidate: &(Signal, SubarrayId), network: &impl NetworkWithBackwardEdges<Node = Aoig>) -> Vec<Instruction> {
        let (signal, result_subarray) = next_candidate;
        let node_id = signal.node_id();

        // quick validity check: ensure all inputs are already present in the required array
        // assert!(network.node(node_id).inputs().iter().all(|input| {
        //     // TODO
        // }));

        let mut next_instructions = vec!();
        // 1. Perform actual operation of the node

        let compute_subarray = if signal.is_inverted() { result_subarray.get_partner_subarray() } else { *result_subarray }; // for inverted signals first compute the noninverted signal in the other subarray

        // println!("EXECUTING {:?}", next_candidate);
        next_instructions.append(&mut self.execute_and_or(node_id, compute_subarray, network));

        // 2. Negate the result (if needed)
        if signal.is_inverted() {
            let mut negate_instructions = self.execute_not(signal, *result_subarray);
            next_instructions.append(&mut negate_instructions);
        }
        next_instructions
    }

    /// Compute `SchedulingPrio` for a given `signal` located in the `subarray`
    /// - used for inserting new candidates
    ///
    /// TODO: write unittest for this function
    fn compute_scheduling_prio_for_node(&self, signal: Signal, subarray: SubarrayId, network: &impl NetworkWithBackwardEdges<Node = Aoig>) -> SchedulingPrio {
        let nr_last_value_uses = network.node(signal.node_id()).inputs() // for each input check whether `id` is the last node using it
            .iter()
            .fold(0, |acc, input| {
                let input_id = Signal::node_id(input);
                let non_computed_outputs: Vec<Id> = network.node_outputs(input_id) // get all other nodes still relying on this input
                    .filter(|out| {

                        let out_signal = Signal::new(*out,false);
                        out_signal != signal && // all output signals except for the current one
                        !(self.comp_state.value_states.contains_key(&(out_signal, subarray))) // that are not yet computed (not rows present in `subarray` holding that value
                    }) // filter for uses of `input` which still rely on it (=those that are not computed yet, except for currently checked node
                    .collect();
                if non_computed_outputs.is_empty() {
                    acc + 1
                } else {
                    acc
                }
            });

        SchedulingPrio {
            nr_last_value_uses,
            nr_src_operands: network.node(signal.node_id()).inputs().len(),
            nr_result_operands:  network.node_outputs(signal.node_id()).collect::<Vec<Id>>().len(),
        }
    }
}

/// Stores the current state of a row at a concrete compilations step
#[derive(Default)] // by default not a compute_row, no live-value and no constant inside row
pub struct RowState {
    /// `compute_rows` are reservered rows which solely exist for performing computations, see [`Compiler::compute_row_activations`]
    is_compute_row: bool,
    /// `None` if the value inside this row is currently not live
    live_value: Option<Signal>,
    /// Mostly 0s/1s (for initializing reference subarray), see [`CompilationState::constant_values`]
    constant: Option<usize>,
}

/// Keep track of current progress of the compilation (eg which rows are used, into which rows data is placed, ...)
pub struct CompilationState {
    /// For each row in the dram-module store its state (whether it's a compute row or if not whether/which value is stored inside it
    dram_state: HashMap<RowAddress, RowState>,
    /// For each subarray it stores the row in which the `Signal` is located
    value_states: HashMap<(Signal, SubarrayId), RowAddress>,
    /// Stores row location of constant
    /// - REMINDER: some constants are stored in fixed rows (!in each subarray), eg 0s and 1s for initializing reference subarray
    constant_values: HashMap<usize, RowAddress>,
    /// List of candidates (ops ready to be issued) prioritized by some metric by which they are scheduled for execution
    /// - NOTE: calculate Nodes `SchedulingPrio` using
    candidates: PriorityQueue<(Signal, SubarrayId), SchedulingPrio>,
    /// For each Subarray store which rows are free (and hence can be used for storing values)
    free_rows_per_subarray: HashMap<SubarrayId, Vec<RowAddress>>,
}

impl CompilationState {
    pub fn new(dram_state: HashMap<RowAddress, RowState>) -> Self {
        Self {
            dram_state,
            value_states: HashMap::new(),
            constant_values: HashMap::new(),
            candidates: PriorityQueue::new(),
            free_rows_per_subarray: HashMap::new(),
        }
    }
}

/// Contains info to order nodes for Instruction Scheduling
/// GOAL: minimize register usage:
///     1. Number of last-value-uses
///     2. Number src operands: ASSUMPTIONS=executing that op reduces value-usage of all of those inputs, higher prob. of last-use in next steps)
///     3. Total number of nodes that have `node` as input-operands
///         - possible extension: weigh result-operands based on how many of their src-operands are already computed
#[derive(PartialEq, Eq, Debug)]
struct SchedulingPrio {
    /// Nr of values which are used last by that node
    nr_last_value_uses: u64,
    /// Number of source operands the Node has
    nr_src_operands: usize,
    /// Number of result operands the Node has
    nr_result_operands: usize,
}

/// To execute the next instruction based on the following criteria:
/// 1. Select candidate which operates on most values which are used last (->to release safe-space right after)
/// 2. IF EQUAL: Select candidate with most successors (->pushes more candidates to select from in next step)
/// 3. IF EQUAL: Select any of the remaining candidates
impl Ord for SchedulingPrio {
    fn cmp(&self, other: &Self) -> Ordering {
        self.nr_last_value_uses.cmp(&other.nr_last_value_uses)
            .then(self.nr_src_operands.cmp(&other.nr_src_operands)) // if `nr_last_value_uses` is equal
            .then(self.nr_result_operands.cmp(&other.nr_result_operands))
    }
}

impl PartialOrd for SchedulingPrio {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[cfg(test)]
mod tests {
    use eggmock::egg::{self, EGraph, Extractor, RecExpr};
    use eggmock::{AoigLanguage, ComputedNetworkWithBackwardEdges, Network};

    use crate::fc_dram::cost_estimation::CompilingCostFunction;

    use super::*; use std::ffi::CString;
    // import all elements from parent-module
    use std::sync::Once;

    // ERROR: `eggmock`-API doesn't allow this..
    // // For data shared among unittests but initalized only once
    // static TEST_DATA: LazyLock<_> = LazyLock::new(|| {
    //     let mut egraph: EGraph<AoigLanguage, ()> = Default::default();
    //     let my_expression: RecExpr<AoigLanguage> = "(and (and a c) (and b c))".parse().unwrap();
    //     let extractor = Extractor::new( &egraph, CompilingCostFunction {});
    //     let ntk = (extractor, vec!(egg::Id::from(9)));
    //
    //     ComputedNetworkWithBackwardEdges::new(&ntk)
    // });

    // ERROR: This also does not work bc of the weird implementation of a network
    // fn simple_egraph() -> ComputedNetworkWithBackwardEdges<'static, (Extractor<'static, CompilingCostFunction, AoigLanguage, ()>, Vec<egg::Id>)> {
    //     let mut egraph: EGraph<AoigLanguage, ()> = Default::default();
    //     let my_expression: RecExpr<AoigLanguage> = "(and (and 1 3) (and 2 3))".parse().unwrap();
    //     egraph.add_expr(&my_expression);
    //     let extractor = Extractor::new( &egraph, CompilingCostFunction {});
    //     let ntk = &(extractor, vec!(egg::Id::from(5)));
    //     ntk.dump();
    //     // Id(5): And([Signal(false, Id(2)), Signal(false, Id(4))])
    //     // Id(4): And([Signal(false, Id(3)), Signal(false, Id(1))])
    //     // Id(1): Input(3)
    //     // Id(3): Input(2)
    //     // Id(2): And([Signal(false, Id(0)), Signal(false, Id(1))])
    //     // Id(0): Input(1)
    //
    //     let ntk_backward = ComputedNetworkWithBackwardEdges::new(ntk);
    //     ntk_backward
    // }

    static INIT: Once = Once::new();

    fn init() -> Compiler {
        INIT.call_once(|| {
            env_logger::init();
        });
        Compiler::new(CompilerSettings {
            print_program: true, verbose: true, print_compilation_stats: false, min_success_rate: 0.999, repetition_fracops: 5, safe_space_rows_per_subarray: 16,
            config_file:  CString::new("<your-path-to>/config/fcdram_hksynx_compute_rows.json").unwrap().as_ptr(),
            do_save_config: false } )
    }

    #[test]
    fn test_input_placement () {

        let mut compiler = init();
        let mut egraph: EGraph<AoigLanguage, ()> = Default::default();

        // Create Input nodes
        let id1 = egraph.add(AoigLanguage::Input(0)); // Id(1)
        let id2 = egraph.add(AoigLanguage::Input(1)); // Id(2)
        let id3 = egraph.add(AoigLanguage::Input(2)); // Id(3)

        // And([Signal(false, Id(2)), Signal(false, Id(3))]) → Id(4)
        let id4 = egraph.add(AoigLanguage::And([id2, id3]));

        // And([Signal(false, Id(1)), Signal(true, Id(3))]) → Id(6)
        let not_id3 = egraph.add(AoigLanguage::Not(id3));
        let id6 = egraph.add(AoigLanguage::And([id1, not_id3]));

        // Or([Signal(false, Id(4)), Signal(false, Id(6))]) → Id(10)
        let id10 = egraph.add(AoigLanguage::Or([id4, id6]));

        let extractor = Extractor::new( &egraph, CompilingCostFunction {});
        let ntk = &(extractor, vec!(id10));
        ntk.dump();

        let ntk_with_backward_edges = ntk.with_backward_edges();

        let program = compiler.compile(&ntk_with_backward_edges);

        println!("{program}");
    }

    #[test]
    fn test_candidate_initialization() {
        let mut compiler = init();

        let mut egraph: EGraph<AoigLanguage, ()> = Default::default();
        let my_expression: RecExpr<AoigLanguage> = "(and (and 1 3) (and 2 3))".parse().unwrap();
        egraph.add_expr(&my_expression);
        let output2 = egraph.add(AoigLanguage::And([eggmock::egg::Id::from(0), eggmock::egg::Id::from(2)])); // additional `And` with one src-operand=input and one non-input src operand
        debug!("EGraph used for candidate-init: {:?}", egraph);
        let extractor = Extractor::new( &egraph, CompilingCostFunction {});
        let ntk = &(extractor, vec!(egg::Id::from(5), output2));
        ntk.dump();
        // Id(5): And([Signal(false, Id(2)), Signal(false, Id(4))])
        // Id(4): And([Signal(false, Id(3)), Signal(false, Id(1))])
        // Id(1): Input(3)
        // Id(3): Input(2)
        // Id(2): And([Signal(false, Id(0)), Signal(false, Id(1))])
        // Id(0): Input(1)

        // act is if one `AND` has already been computed -> other and (`Id(2)`) should be the only candidate left
        // compiler.comp_state.value_states.insert(Signal::new(eggmock::Id::from(4), false), ValueState{ is_computed: true, row_location: None });
        //
        // let ntk_backward = ComputedNetworkWithBackwardEdges::new(ntk);
        //
        // compiler.init_candidates(&ntk_backward);
        // let is_candidate_ids: HashSet<Signal> = compiler.comp_state.candidates.iter().map(|(id,_)| *id).collect();
        // let should_candidate_ids: HashSet<Signal> = HashSet::from([Signal::new( eggmock::Id::from(2), false), Signal::new(eggmock::Id::from(4), false)]);
        // assert_eq!( is_candidate_ids, should_candidate_ids);

        // TODO: test-case with node that relies on one input src-operand and one non-input (intermediate node) src-operand
    }

    #[test]
    fn test_new_candidates() {
        let mut compiler = init();

    }

    #[test]
    fn test_compute_scheduling_prio_for_node() {
        let mut compiler = init();

        let mut egraph: EGraph<AoigLanguage, ()> = Default::default();
        let my_expression: RecExpr<AoigLanguage> = "(and (and 1 3) (and 2 3))".parse().unwrap();
        egraph.add_expr(&my_expression);
        let extractor = Extractor::new( &egraph, CompilingCostFunction {});
        let ntk = &(extractor, vec!(egg::Id::from(5)));
        ntk.dump();
        // Id(5): And([Signal(false, Id(2)), Signal(false, Id(4))])
        // Id(4): And([Signal(false, Id(3)), Signal(false, Id(1))])
        // Id(1): Input(3)
        // Id(3): Input(2)
        // Id(2): And([Signal(false, Id(0)), Signal(false, Id(1))])
        // Id(0): Input(1)

        // act is if one `AND` has already been computed -> other and (`Id(2)`) should be the only candidate left
        // compiler.comp_state.value_states.insert(Signal::new(eggmock::Id::from(2), false), ValueState{ is_computed: true, row_location: None });
        //
        // let ntk_backward = ComputedNetworkWithBackwardEdges::new(ntk);
        //
        // let scheduling_prio = compiler.compute_scheduling_prio_for_node(Signal::new(eggmock::Id::from(4), false),  &ntk_backward);
        // assert_eq!(scheduling_prio, SchedulingPrio { nr_last_value_uses: 2, nr_src_operands: 2, nr_result_operands: 1 }  );

    }

    // TODO
    #[test]
    fn test_select_compute_and_ref_subarray() {
        let compiler = init();
        // let (selected_subarray, _) = compiler.select_compute_and_ref_subarray(vec!(RowAddress(0b1_000_000_000), RowAddress(0b1_000_010_000), RowAddress(0b111_000_000_000), RowAddress(0b10_100_000_000),));
        // assert_eq!(selected_subarray, 0b1_000_000_000);
    }

    #[ignore]
    fn test_program_validity() {
        // 1. test that no APA activates a safe-space row

        // ..
    }
}
