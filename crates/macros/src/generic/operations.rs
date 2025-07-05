use std::{
    collections::{HashMap, hash_map::Entry},
    str::FromStr,
};

use lime_generic_def::{Function, Gate, InputIndices, OperationType};
use quote::{ToTokens, quote};
use syn::{Error, Result};

use crate::generic::ast::IntOrStar;

use super::{
    CellType, OperandsValue,
    ast::{self},
    krate,
    operands::NamedOperands,
};

pub struct Operations(pub HashMap<String, OperationType<CellType>>);

impl Operations {
    pub fn new(operations: &NamedOperands, ast: &ast::Architecture) -> Result<Self> {
        let mut result = HashMap::new();
        for operation in ast.inner.operations.value.iter() {
            let name = operation.name.to_string();
            let Entry::Vacant(entry) = result.entry(name) else {
                return Err(Error::new(
                    operation.name.span(),
                    "duplicate operation name",
                ));
            };
            let input = operations.by_ident(&operation.input.value)?;
            let input_target = match operation
                .input_target_idx
                .0
                .as_ref()
                .map(|bracketed| &bracketed.value)
            {
                None => None,
                Some(IntOrStar::Int(idx_lit)) => {
                    let idx = idx_lit.base10_parse()?;
                    if input.arity().is_none_or(|arity| idx >= arity) {
                        return Err(Error::new(idx_lit.span(), "index out of bounds"));
                    }
                    Some(InputIndices::Index(idx))
                }
                Some(IntOrStar::Star(_)) => Some(InputIndices::All),
            };
            let function = (&operation.function).try_into()?;
            entry.insert(OperationType {
                name: operation.name.to_string().into(),
                input,
                input_override: input_target,
                function,
            });
        }
        Ok(Self(result))
    }
}

impl ToTokens for Operations {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let krate = krate();
        let operations = self.0.values().map(OperationTypeValue);
        tokens.extend(quote! {
            #krate::Operations::new(vec![
                #(#operations),*
            ])
        });
    }
}

struct OperationTypeValue<'a>(&'a OperationType<CellType>);

impl ToTokens for OperationTypeValue<'_> {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let OperationType {
            name,
            input,
            input_override,
            function,
        } = &self.0;
        let (input, input_override, function) = (
            OperandsValue(input),
            InputIndicesValue(input_override),
            FunctionValue(*function),
        );
        let krate = krate();
        tokens.extend(quote! {
            #krate::OperationType {
                name: #name.into(),
                input: #input,
                input_override: #input_override,
                function: #function
            }
        });
    }
}

struct InputIndicesValue<'a>(&'a Option<InputIndices>);

impl ToTokens for InputIndicesValue<'_> {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let krate = krate();
        tokens.extend(match self.0 {
            None => quote!(None),
            Some(InputIndices::All) => quote!(Some(#krate::InputIndices::All)),
            Some(InputIndices::Index(idx)) => quote!(Some(#krate::InputIndices::Index(#idx))),
        })
    }
}

impl TryFrom<&ast::Function> for Function {
    type Error = Error;

    fn try_from(value: &ast::Function) -> Result<Self> {
        Ok(Self {
            gate: Gate::try_from(&value.gate)?,
            inverted: value.inverted.is_some(),
        })
    }
}

struct FunctionValue(Function);

impl ToTokens for FunctionValue {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let krate = krate();
        let Function { inverted, gate } = self.0;
        let gate = GateValue(gate);
        tokens.extend(quote! {
            #krate::Function {
                gate: #gate,
                inverted: #inverted,
            }
        });
    }
}

impl TryFrom<&ast::BoolOrIdent> for Gate {
    type Error = Error;

    fn try_from(value: &ast::BoolOrIdent) -> Result<Self> {
        match value {
            ast::BoolOrIdent::Bool(lit) => Ok(Self::Constant(lit.value)),
            ast::BoolOrIdent::Ident(ident) => Gate::from_str(ident.to_string().as_str())
                .map_err(|_| Error::new(ident.span(), "unknown gate")),
        }
    }
}

struct GateValue(Gate);

impl ToTokens for GateValue {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let variant = match self.0 {
            Gate::And => quote!(And),
            Gate::Maj => quote!(Maj),
            Gate::Constant(c) => quote!(Constant(#c)),
        };
        let krate = krate();
        tokens.extend(quote!(#krate::Gate::#variant));
    }
}
