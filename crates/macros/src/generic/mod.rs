mod ast;
mod cells;
mod operands;
mod operations;
mod outputs;

use std::rc::Rc;

use convert_case::{Case, Casing};
use proc_macro2::TokenStream;
use quote::{ToTokens, format_ident, quote};
use syn::{Path, Result, parse_quote};

pub use self::{cells::*, operands::*, operations::*, outputs::*};

pub struct Architecture {
    pub ast: Rc<ast::Architecture>,
    pub cells: Cells,
    pub operands: NamedOperands,
    pub operations: Operations,
    pub outputs: Outputs,
}

impl TryFrom<ast::Architecture> for Architecture {
    type Error = syn::Error;

    fn try_from(ast: ast::Architecture) -> Result<Self> {
        let ast = Rc::new(ast);
        let cells = Cells::from_ast(ast.clone())?;
        let operands = NamedOperands::new(&cells, &ast)?;
        let operations = Operations::new(&operands, &ast)?;
        let outputs = Outputs::new(&operands, &ast)?;
        Ok(Self {
            ast,
            cells,
            operands,
            operations,
            outputs,
        })
    }
}

impl ToTokens for Architecture {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let vis = &self.ast.vis;
        let name = &self.ast.name;
        let krate = krate();
        let ct = cell_type_enum_name(name);
        let operands_fns = self.operands.keys().map(|operand| {
            format_ident!("operands_{operand}", operand = operand.to_case(Case::Snake))
        });
        let operands = self.operands.values().map(OperandsValue);
        let cells = &self.cells;
        let operations = &self.operations;
        let outputs = &self.outputs;

        tokens.extend(quote! {
            #cells

            #vis struct #name;

            impl #name {
                #(
                    #vis fn #operands_fns() -> #krate::Operands<#ct> {
                        #operands
                    }
                )*
                #vis fn operations() -> #krate::Operations<#ct> {
                    #operations
                }
                #vis fn outputs() -> #krate::Outputs<#ct> {
                    #outputs
                }
                #vis fn new() -> #krate::Architecture<#ct> {
                    #krate::Architecture::new(Self::operations(), Self::outputs())
                }
            }
        });
    }
}

pub fn define_generic_architecture(item: TokenStream) -> Result<TokenStream> {
    let ast: ast::Architecture = syn::parse2(item)?;
    let arch = Architecture::try_from(ast)?;
    Ok(arch.into_token_stream())
}

pub fn krate() -> Path {
    parse_quote!(lime_generic_def)
}
