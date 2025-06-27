mod generic;

use proc_macro::TokenStream;

#[proc_macro]
pub fn define_generic_architecture(item: TokenStream) -> TokenStream {
    match generic::define_generic_architecture(item.into()) {
        Err(err) => err.to_compile_error().into(),
        Ok(stream) => stream.into(),
    }
}
