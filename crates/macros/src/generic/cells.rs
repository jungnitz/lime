use std::{collections::BTreeMap, rc::Rc};

use derive_more::Deref;
use lime_generic_def::{CellIndex, OperandType};
use proc_macro2::{Span, TokenStream};
use quote::{ToTokens, format_ident, quote};
use syn::{Error, Ident, Result};

use crate::generic::ast::Bracketed;

use super::{ast, ast::BoolOrIdent, krate};

#[derive(Deref)]
pub struct Cells {
    #[deref]
    entries: BTreeMap<String, Option<u32>>,
    ast: Rc<ast::Architecture>,
}

impl Cells {
    pub fn from_ast(ast: Rc<ast::Architecture>) -> Result<Self> {
        let tuple = &ast.inner.cells;
        let mut entries = BTreeMap::new();
        for def in tuple.value.iter() {
            let prev = entries.insert(
                def.name.to_string(),
                def.num.as_ref().map(|lit| lit.base10_parse()).transpose()?,
            );
            if prev.is_some() {
                return Err(syn::Error::new(def.name.span(), "duplicate cell type name"));
            }
        }
        Ok(Self { entries, ast })
    }

    pub fn new_operand_type(&self, ast: &ast::OperandType) -> Result<OperandType<CellType>> {
        let (typ, index) = match &ast.name {
            BoolOrIdent::Bool(lit) => {
                if let Some(Bracketed { bracket, .. }) = ast.index.0 {
                    return Err(Error::new(
                        bracket.span.join(),
                        "cannot index constant value",
                    ));
                }
                (
                    CellType::constant(self.ast.clone(), lit.span),
                    Some(lit.value() as CellIndex),
                )
            }
            BoolOrIdent::Ident(ident) => {
                let typ = CellType::new(self.ast.clone(), ident);
                let arity = if let Some(name) = &typ.name {
                    if let Some(count) = self.get(name) {
                        *count
                    } else {
                        return Err(Error::new(ident.span(), "no such cell type"));
                    }
                } else {
                    Some(2) // constant
                };
                let index = ast
                    .index
                    .0
                    .as_ref()
                    .map(
                        |Bracketed {
                             value: index_lit, ..
                         }|
                         -> Result<CellIndex> {
                            let index = index_lit.base10_parse()?;
                            if let Some(count) = arity
                                && index >= count
                            {
                                return Err(Error::new(
                                    index_lit.span(),
                                    "cell index out of bounds for this type",
                                ));
                            }
                            Ok(index)
                        },
                    )
                    .transpose()?;
                (typ, index)
            }
        };
        Ok(OperandType {
            typ,
            inverted: ast.invert.is_some(),
            index,
        })
    }
}

impl ToTokens for Cells {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let name_idents: Vec<_> = self
            .entries
            .keys()
            .map(|name| format_ident!("{name}"))
            .collect();
        let name_strs = self.entries.keys();
        let arities = self.entries.values().map(|arity| match arity {
            None => quote!(None),
            Some(arity) => quote!(Some(#arity)),
        });
        let name = cell_type_enum_name(&self.ast.name);
        let vis = &self.ast.vis;
        let krate = krate();
        tokens.extend(quote! {
            #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
            #vis enum #name {
                Constant,
                #(#name_idents),*
            }
            impl #krate::CellType for #name {
                const CONSTANT: Self = Self::Constant;
                fn count(self) -> Option<#krate::CellIndex> {
                    match self {
                        Self::Constant => Some(2),
                        #(Self::#name_idents => #arities),*
                    }
                }
                fn name(self) -> &'static str {
                    match self {
                        Self::Constant => "bool",
                        #(Self::#name_idents => #name_strs),*
                    }
                }
            }
            impl std::fmt::Display for #name {
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    write!(f, "{}", #krate::CellType::name(*self))
                }
            }
        });
    }
}

#[derive(Clone)]
pub struct CellType {
    ast: Rc<ast::Architecture>,
    span: Span,
    name: Option<String>,
}

impl CellType {
    pub fn constant(ast: Rc<ast::Architecture>, span: Span) -> Self {
        Self {
            ast,
            span,
            name: None,
        }
    }
    pub fn new(ast: Rc<ast::Architecture>, name: &Ident) -> Self {
        let str = name.to_string();
        let span = name.span();
        let name = match str.as_str() {
            "bool" => None,
            _ => Some(str),
        };
        Self { ast, span, name }
    }
    pub fn variant_name(&self) -> Ident {
        match &self.name {
            None => Ident::new("Constant", self.span),
            Some(name) => Ident::new(name, self.span),
        }
    }
}

impl ToTokens for CellType {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let typ = cell_type_enum_name(&self.ast.name);
        let variant = self.variant_name();
        tokens.extend(quote!(#typ :: #variant));
    }
}

pub fn cell_type_enum_name(arch_name: &Ident) -> Ident {
    Ident::new(&format!("{arch_name}CellType"), arch_name.span())
}
