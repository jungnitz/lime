mod generic;

use proc_macro::TokenStream;

#[proc_macro]
pub fn define_generic_architecture(item: TokenStream) -> TokenStream {
    generic::define_generic_architecture(item.into()).into()
}
