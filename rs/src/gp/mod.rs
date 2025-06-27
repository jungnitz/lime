#![allow(unused)]

pub use lime_generic_def::*;
mod dsl;
mod ops;
mod program;
mod state;

pub use dsl::*;

lime_macros::define_generic_architecture! {
    Ambit {
        cells ([T], [D; 3], [A; 3]),
        operands (
            NONE = [()],
            OP = [(T, D, A, A)],
            test = [
                (T, T, T, T),
                (!A[2], T[1], true, false),
                ...OP
            ],
            OP3 = [T],
        ),
        operations (
            WHAT = (OP3 := and -> and),
            WHAT2 = (OP := (and, !and, and, maj) -> and),
            WHAT3 = (OP := (and, !and, maj, !and) -> and),
        ),
        output (
            NONE,
            OP,
        )
    }
}

fn test() {
    let arch = Ambit::new();
}
