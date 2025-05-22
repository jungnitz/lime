pub struct Operand(pub u64);

pub struct LogicFunction;

pub struct OperandClass {
    pub name: &'static str,
    pub members: Vec<Operand>,
    pub infinite: bool,
}

pub struct OperationType {
    pub inputs: Vec<OperationInput>,
    pub outputs: OperationOutput,
}

pub struct OperationInput {
    class: OperandClass,
    output: OperationInputResult,
}

pub enum OperationInputResult {
    Destroyed,
    Unchanged,
    Function(LogicFunction),
}

pub struct OperationOutput {
    class: OperandClass,
    function: LogicFunction,
}

pub struct Operation {}
