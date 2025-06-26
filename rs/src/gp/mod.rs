#![allow(unused)]

mod def;
mod dsl;
mod ops;
mod program;
mod state;

pub use {def::*, dsl::*};

lime_macros::define_generic_architecture! {
    Ambit {
        cells ([T], [D; 3], [T])
    }
}
