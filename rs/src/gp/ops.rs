use super::{Architecture, OperandSet, OperandType, OperationType};

pub fn copy<A: Architecture>(arch: &A, from: A::Cell, to: A::Cell, invert: bool) {
    for operation in arch.operations() {
        match operation.input {
            OperandSet::Cross(cross) => copy_using_cross(arch, from, to, invert, operation, cross),
            OperandSet::Set(set) => copy_using_set(arch, from, to, invert, operation, set),
        }
    }
}

pub fn copy_using_cross<A: Architecture>(
    arch: &A,
    from: A::Cell,
    to: A::Cell,
    invert: bool,
    operation: &OperationType<A>,
    cross: &[OperandType<A>],
) {
}

pub fn copy_using_set<A: Architecture>(
    arch: &A,
    from: A::Cell,
    to: A::Cell,
    invert: bool,
    operation: &OperationType<A>,
    set: &[&[OperandType<A>]],
) {
}
