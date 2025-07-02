use super::{Architecture, BitwiseOperand, BitwiseRow, Row, Rows};
use eggmock::{Id, Mig, NetworkWithBackwardEdges, Signal};
use std::fmt::{Display, Formatter};
use std::ops::{Deref, DerefMut};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Address {
    In(u64),
    Out(u64),
    Spill(u32),
    Const(bool),
    Bitwise(BitwiseAddress),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum SingleRowAddress {
    In(u64),
    Out(u64),
    Spill(u32),
    Const(bool),
    Bitwise(BitwiseOperand),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum BitwiseAddress {
    Single(BitwiseOperand),
    Multiple(usize),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Instruction {
    AAP(Address, Address),
    AP(Address),
}

#[derive(Debug, Clone)]
pub struct Program<'a> {
    pub architecture: &'a Architecture,
    pub instructions: Vec<Instruction>,
}

#[derive(Debug, Clone)]
pub struct ProgramState<'a> {
    program: Program<'a>,
    rows: Rows<'a>,
}

impl<'a> Program<'a> {
    pub fn new(architecture: &'a Architecture, instructions: Vec<Instruction>) -> Self {
        Self {
            architecture,
            instructions,
        }
    }
}

impl Instruction {
    pub fn used_addresses<'a>(
        &self,
        architecture: &'a Architecture,
    ) -> impl Iterator<Item = SingleRowAddress> + 'a + use<'a> {
        let from = match self {
            Instruction::AAP(from, _) => from,
            Instruction::AP(op) => op,
        }
        .row_addresses(architecture);
        let to = match self {
            Instruction::AAP(_, to) => Some(*to),
            _ => None,
        }
        .into_iter()
        .flat_map(|addr| addr.row_addresses(architecture));
        from.chain(to)
    }

    pub fn input_operands<'a>(
        &self,
        architecture: &'a Architecture,
    ) -> impl Iterator<Item = SingleRowAddress> + 'a {
        let from = match self {
            Instruction::AAP(from, _) => from,
            Instruction::AP(op) => op,
        };
        from.row_addresses(architecture)
    }

    pub fn overridden_rows<'a>(
        &self,
        architecture: &'a Architecture,
    ) -> impl Iterator<Item = Row> + 'a {
        let first = *match self {
            Instruction::AP(a) => a,
            Instruction::AAP(a, _) => a,
        };
        let first = match first {
            Address::Bitwise(BitwiseAddress::Multiple(idx)) => {
                architecture.multi_activations[idx].as_slice()
            }
            _ => &[],
        }
        .iter()
        .map(|op| Row::Bitwise(op.row()));

        let second = match self {
            Instruction::AP(_) => None,
            Instruction::AAP(_, a) => Some(a.row_addresses(architecture).map(|addr| addr.row())),
        }
        .into_iter()
        .flatten();

        first.chain(second)
    }
}

impl<'a> ProgramState<'a> {
    pub fn new(
        architecture: &'a Architecture,
        network: &impl NetworkWithBackwardEdges<Node = Mig>,
    ) -> Self {
        Self {
            program: Program::new(architecture, Vec::new()),
            rows: Rows::new(network, architecture),
        }
    }

    pub fn maj(&mut self, op: usize, out_signal: Signal, out_address: Option<Address>) {
        let operands = &self.architecture.multi_activations[op];
        for operand in operands {
            self.set_signal(SingleRowAddress::Bitwise(*operand), out_signal);
        }
        let instruction = match out_address {
            Some(out) => Instruction::AAP(BitwiseAddress::Multiple(op).into(), out),
            None => Instruction::AP(BitwiseAddress::Multiple(op).into()),
        };
        self.instructions.push(instruction)
    }

    pub fn signal_copy(&mut self, signal: Signal, target: SingleRowAddress, intermediate_dcc: u8) {
        {
            // if any row contains the signal, then this is easy, simply copy the row into the
            // target operand
            let signal_row = self.rows.get_rows(signal).next();
            if let Some(signal_row) = signal_row {
                self.set_signal(target, signal);
                self.instructions
                    .push(Instruction::AAP(signal_row.into(), target.into()));
                return;
            }
        }
        // otherwise we need to search the inverted signal and take a DCC row if possible, otherwise
        // use the intermediate DCC to invert
        let mut inverted_signal_row = None;
        for row in self.rows.get_rows(signal.invert()) {
            if let Row::Bitwise(BitwiseRow::DCC(_)) = row {
                inverted_signal_row = Some(row);
                break;
            }
            inverted_signal_row = Some(row);
        }
        let inverted_signal_row =
            inverted_signal_row.expect("inverted signal row should be present");

        if let SingleRowAddress::Bitwise(BitwiseOperand::DCC { inverted, index }) = target {
            // if the target is a DCC operand, we can simply copy over the signal
            self.set_signal(target, signal);
            self.instructions.push(Instruction::AAP(
                inverted_signal_row.into(),
                BitwiseOperand::DCC {
                    inverted: !inverted,
                    index,
                }
                .into(),
            ));
            return;
        }

        if let Row::Bitwise(BitwiseRow::DCC(dcc)) = inverted_signal_row {
            // alrighty, that's great, the inverted DCC row contains our signal
            // let's copy that over
            self.set_signal(target, signal);
            self.instructions.push(Instruction::AAP(
                BitwiseOperand::DCC {
                    inverted: true,
                    index: dcc,
                }
                .into(),
                target.into(),
            ));
            return;
        }
        // this is the very sad case in which we have to use the intermediate DCC row to create the
        // signal from its inverse
        // first, copy the inverted signal from the signal_row to the DCC row
        let intermediate_dcc_addr = SingleRowAddress::Bitwise(BitwiseOperand::DCC {
            inverted: false,
            index: intermediate_dcc,
        });
        self.set_signal(intermediate_dcc_addr, signal.invert());
        self.instructions.push(Instruction::AAP(
            inverted_signal_row.into(),
            intermediate_dcc_addr.into(),
        ));

        // then copy the signal from the inverted intermediate DCC row into our target operand
        let inv_intermediate_dcc_operand = BitwiseOperand::DCC {
            inverted: true,
            index: intermediate_dcc,
        };
        self.set_signal(target, signal);
        self.instructions.push(Instruction::AAP(
            inv_intermediate_dcc_operand.into(),
            target.into(),
        ));
    }

    /// Sets the value of the operand in `self.rows` to the given signal. If that removes the last
    /// reference to the node of the previous signal of the operator, insert spill code for the
    /// previous signal
    /// **ALWAYS** call this before inserting the actual instruction, otherwise the spill code will
    /// spill the wrong value
    fn set_signal(&mut self, address: SingleRowAddress, signal: Signal) {
        if let Some(previous_signal) = self.rows.set_signal(address, signal) {
            if !self.rows.contains_id(previous_signal.node_id()) {
                let spill_id = self.rows.add_spill(previous_signal);
                self.instructions
                    .push(Instruction::AAP(address.into(), Address::Spill(spill_id)));
            }
        }
    }

    pub fn free_id_rows(&mut self, id: Id) {
        self.rows.free_id_rows(id);
    }

    pub fn rows(&self) -> &Rows {
        &self.rows
    }
}

impl Address {
    pub fn as_single_row(&self) -> Option<SingleRowAddress> {
        match self {
            Address::In(i) => Some(SingleRowAddress::In(*i)),
            Address::Out(i) => Some(SingleRowAddress::Out(*i)),
            Address::Spill(i) => Some(SingleRowAddress::Spill(*i)),
            Address::Const(v) => Some(SingleRowAddress::Const(*v)),
            Address::Bitwise(BitwiseAddress::Single(op)) => Some(SingleRowAddress::Bitwise(*op)),
            Address::Bitwise(BitwiseAddress::Multiple(_)) => None,
        }
    }
    pub fn row_addresses<'a>(
        &self,
        architecture: &'a Architecture,
    ) -> impl Iterator<Item = SingleRowAddress> + use<'a> {
        let single = self.as_single_row().into_iter();
        let multi = match self {
            Address::Bitwise(BitwiseAddress::Multiple(i)) => {
                architecture.multi_activations[*i].as_slice()
            }
            _ => &[],
        }
        .iter()
        .map(|op| SingleRowAddress::Bitwise(*op));
        single.chain(multi)
    }
}

impl SingleRowAddress {
    pub fn row(&self) -> Row {
        match self {
            SingleRowAddress::In(i) => Row::In(*i),
            SingleRowAddress::Out(i) => Row::Out(*i),
            SingleRowAddress::Spill(i) => Row::Spill(*i),
            SingleRowAddress::Const(i) => Row::Const(*i),
            SingleRowAddress::Bitwise(o) => o.row().into(),
        }
    }
    pub fn inverted(&self) -> bool {
        match self {
            SingleRowAddress::Bitwise(o) => o.inverted(),
            _ => false,
        }
    }
}

impl<'a> Deref for ProgramState<'a> {
    type Target = Program<'a>;

    fn deref(&self) -> &Self::Target {
        &self.program
    }
}

impl DerefMut for ProgramState<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.program
    }
}

impl<'a> From<ProgramState<'a>> for Program<'a> {
    fn from(value: ProgramState<'a>) -> Self {
        value.program
    }
}

impl From<BitwiseAddress> for Address {
    fn from(value: BitwiseAddress) -> Self {
        Self::Bitwise(value)
    }
}

impl From<BitwiseOperand> for BitwiseAddress {
    fn from(value: BitwiseOperand) -> Self {
        Self::Single(value)
    }
}

impl From<BitwiseOperand> for Address {
    fn from(value: BitwiseOperand) -> Self {
        Self::Bitwise(value.into())
    }
}

impl From<BitwiseOperand> for SingleRowAddress {
    fn from(value: BitwiseOperand) -> Self {
        Self::Bitwise(value)
    }
}

impl From<BitwiseRow> for BitwiseOperand {
    fn from(value: BitwiseRow) -> Self {
        match value {
            BitwiseRow::T(t) => BitwiseOperand::T(t),
            BitwiseRow::DCC(dcc) => BitwiseOperand::DCC {
                index: dcc,
                inverted: false,
            },
        }
    }
}

impl From<Row> for SingleRowAddress {
    fn from(value: Row) -> Self {
        match value {
            Row::In(i) => SingleRowAddress::In(i),
            Row::Out(i) => SingleRowAddress::Out(i),
            Row::Spill(i) => SingleRowAddress::Spill(i),
            Row::Const(c) => SingleRowAddress::Const(c),
            Row::Bitwise(b) => SingleRowAddress::Bitwise(b.into()),
        }
    }
}

impl From<Row> for Address {
    fn from(value: Row) -> Self {
        Self::from(SingleRowAddress::from(value))
    }
}

impl From<SingleRowAddress> for Address {
    fn from(value: SingleRowAddress) -> Self {
        match value {
            SingleRowAddress::In(i) => Self::In(i),
            SingleRowAddress::Out(i) => Self::Out(i),
            SingleRowAddress::Spill(i) => Self::Spill(i),
            SingleRowAddress::Const(c) => Self::Const(c),
            SingleRowAddress::Bitwise(o) => Self::Bitwise(BitwiseAddress::Single(o)),
        }
    }
}

impl Display for Program<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let write_operand = |f: &mut Formatter<'_>, o: &BitwiseOperand| -> std::fmt::Result {
            match o {
                BitwiseOperand::T(t) => write!(f, "T{t}"),
                BitwiseOperand::DCC { inverted, index } => {
                    if *inverted {
                        write!(f, "~DCC{index}")
                    } else {
                        write!(f, "DCC{index}")
                    }
                }
            }
        };
        let write_address = |f: &mut Formatter<'_>, a: &Address| -> std::fmt::Result {
            match a {
                Address::In(i) => write!(f, "I{i}"),
                Address::Out(i) => write!(f, "O{i}"),
                Address::Spill(i) => write!(f, "S{i}"),
                Address::Const(c) => write!(f, "C{}", if *c { "1" } else { "0" }),
                Address::Bitwise(b) => match b {
                    BitwiseAddress::Single(o) => write_operand(f, o),
                    BitwiseAddress::Multiple(id) => {
                        let operands = &self.architecture.multi_activations[*id];
                        for i in 0..operands.len() {
                            if i == 0 {
                                write!(f, "[")?;
                            } else {
                                write!(f, ", ")?;
                            }
                            write_operand(f, &operands[i])?;
                            if i == operands.len() - 1 {
                                write!(f, "]")?;
                            }
                        }
                        Ok(())
                    }
                },
            }
        };

        for instruction in &self.instructions {
            match instruction {
                Instruction::AAP(a, b) => {
                    write!(f, "AAP ")?;
                    write_address(f, a)?;
                    write!(f, " ")?;
                    write_address(f, b)?;
                    writeln!(f)?;
                }
                Instruction::AP(a) => {
                    write!(f, "AP ")?;
                    write_address(f, a)?;
                    writeln!(f)?;
                }
            }
        }
        Ok(())
    }
}
