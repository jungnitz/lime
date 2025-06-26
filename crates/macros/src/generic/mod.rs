use proc_macro2::TokenStream;

use crate::generic::ast::Architecture;

mod ast;

pub fn define_generic_architecture(item: TokenStream) -> TokenStream {
    let arch: Architecture = match syn::parse2(item) {
        Ok(a) => a,
        Err(er) => return er.to_compile_error(),
    };
    todo!("{arch:?}")
}
