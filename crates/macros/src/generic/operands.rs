use derive_more::{Deref, DerefMut};
use itertools::Itertools;
use lime_generic_def::{NaryOperands, OperandTuple, OperandType, Operands, TupleOperands};
use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use std::collections::BTreeMap;
use syn::{Error, Ident, Result};

use super::{
    CellType, Cells, ast,
    ast::{NameAndOperands, OperandTuplesElement},
    krate,
};

#[derive(Deref, DerefMut)]
pub struct NamedOperands(pub BTreeMap<String, Operands<CellType>>);

impl NamedOperands {
    pub fn new(cells: &Cells, ast: &ast::Architecture) -> Result<Self> {
        let mut result = BTreeMap::new();
        for NameAndOperands { name, operands, .. } in &ast.inner.operands.elements {
            let operands = match operands {
                ast::Operands::Tuples(tuples) => {
                    let mut vec = Vec::new();
                    let mut arity = None;
                    for element in tuples {
                        match element {
                            OperandTuplesElement::Tuple(tuple) => {
                                if let Some(arity) = arity
                                    && tuple.elements.len() != arity
                                {
                                    return Err(Error::new(
                                        tuple.paren.span.join(),
                                        "tuple does not match arity with previous tuples",
                                    ));
                                }
                                arity = Some(tuple.elements.len());
                                vec.push(OperandTuple::new(
                                    tuple
                                        .elements
                                        .iter()
                                        .map(|typ| cells.new_operand_type(typ))
                                        .try_collect()?,
                                ))
                            }
                            OperandTuplesElement::Ref(name) => {
                                if let Some(operands) = result.get(name.to_string().as_str()) {
                                    let Operands::Tuples(tuples) = operands else {
                                        return Err(Error::new(
                                            name.span(),
                                            "can only expand tuple operands",
                                        ));
                                    };
                                    if arity.is_some_and(|arity| arity != tuples.arity()) {
                                        return Err(Error::new(
                                            name.span(),
                                            "arity of referenced operands does not match",
                                        ));
                                    }
                                    arity = Some(tuples.arity());
                                    vec.extend(tuples.iter().cloned());
                                } else {
                                    return Err(Error::new(name.span(), "unknown operands name"));
                                }
                            }
                        }
                    }
                    Operands::Tuples(TupleOperands::new(vec))
                }
                ast::Operands::Nary(typ) => {
                    Operands::Nary(NaryOperands(cells.new_operand_type(typ)?))
                }
            };
            if result.insert(name.to_string(), operands).is_some() {
                return Err(Error::new(name.span(), "duplicate operands name"));
            }
        }
        Ok(Self(result))
    }
    pub fn by_ident(&self, name: &Ident) -> Result<Operands<CellType>> {
        self.get(&name.to_string())
            .cloned()
            .ok_or_else(|| Error::new(name.span(), "unknown operands"))
    }
}

pub struct OperandsValue<'a>(pub &'a Operands<CellType>);

impl ToTokens for OperandsValue<'_> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let krate = krate();
        match &self.0 {
            Operands::Nary(nary) => {
                let inner = NaryOperandsValue(nary);
                tokens.extend(quote!(#krate::Operands::Nary(#inner)));
            }
            Operands::Tuples(tuples) => {
                let inner = TupleOperandsValue(tuples);
                tokens.extend(quote!(#krate::Operands::Tuples(#inner)));
            }
        }
    }
}

struct NaryOperandsValue<'a>(&'a NaryOperands<CellType>);

impl ToTokens for NaryOperandsValue<'_> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let krate = krate();
        let inner = OperandTypeValue(&self.0.0);
        tokens.extend(quote!(#krate::NaryOperands(#inner)));
    }
}

struct TupleOperandsValue<'a>(&'a TupleOperands<CellType>);

impl ToTokens for TupleOperandsValue<'_> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let krate = krate();
        let tuples = self.0.iter().map(OperandTupleValue);
        tokens.extend(quote! {
            #krate::TupleOperands::new(vec![
                #(#tuples),*
            ])
        });
    }
}

struct OperandTupleValue<'a>(&'a OperandTuple<CellType>);

impl ToTokens for OperandTupleValue<'_> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let krate = krate();
        let types = self.0.iter().map(OperandTypeValue);
        tokens.extend(quote! {
            #krate::OperandTuple::new(vec![
                #(#types),*
            ])
        });
    }
}

struct OperandTypeValue<'a>(&'a OperandType<CellType>);

impl ToTokens for OperandTypeValue<'_> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let krate = krate();
        let OperandType {
            typ,
            inverted,
            index,
        } = &self.0;
        let index = index.map_or_else(|| quote!(None), |idx| quote!(Some(#idx)));
        tokens.extend(quote! {
            #krate::OperandType {
                typ: #typ,
                inverted: #inverted,
                index: #index,
            }
        });
    }
}
