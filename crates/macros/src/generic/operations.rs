use std::{
    collections::{HashMap, hash_map::Entry},
    str::FromStr,
};

use itertools::Itertools;
use lime_generic_def::{Function, Gate, InputResult, InputResults, OperationType};
use quote::{ToTokens, quote};
use syn::{Error, Result};

use crate::generic::{
    CellType, OperandsValue,
    ast::{self},
    krate,
    operands::NamedOperands,
};

pub struct Operations(pub HashMap<String, OperationType<CellType>>);

impl Operations {
    pub fn new(operations: &NamedOperands, ast: &ast::Architecture) -> Result<Self> {
        let mut result = HashMap::new();
        for operation in &ast.inner.operations.elements {
            let name = operation.name.to_string();
            let Entry::Vacant(entry) = result.entry(name) else {
                return Err(Error::new(
                    operation.name.span(),
                    "duplicate operation name",
                ));
            };
            let input = operations.by_ident(&operation.input)?;
            let input_results = operation
                .input_results
                .as_ref()
                .map(InputResults::try_from)
                .transpose()?
                .unwrap_or(InputResults::new_unchanged());
            if let Some(result_arity) = input_results.arity()
                && input.arity() != Some(result_arity)
            {
                return Err(Error::new(
                    operation.input.span(),
                    "operands arity does not match input result arity",
                ));
            }
            let output = operation
                .output_function
                .as_ref()
                .map(Function::try_from)
                .transpose()?;
            entry.insert(OperationType {
                name: operation.name.to_string().into(),
                input,
                input_results,
                output,
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
            input_results,
            output,
        } = &self.0;
        let (input, input_results) = (OperandsValue(input), InputResultsValue(input_results));
        let output = match output {
            None => quote!(None),
            Some(output) => {
                let output = FunctionValue(*output);
                quote!(Some(#output))
            }
        };
        let krate = krate();
        tokens.extend(quote! {
            #krate::OperationType {
                name: #name.into(),
                input: #input,
                input_results: #input_results,
                output: #output,
            }
        });
    }
}

impl TryFrom<&ast::InputResults> for InputResults {
    type Error = Error;

    fn try_from(value: &ast::InputResults) -> Result<Self> {
        Ok(match value {
            ast::InputResults::All(result) => InputResults::all(result.try_into()?),
            ast::InputResults::Tuple(tuple) => InputResults::new(
                tuple
                    .elements
                    .iter()
                    .map(|result| result.try_into())
                    .try_collect()?,
            ),
        })
    }
}

struct InputResultsValue<'a>(&'a InputResults);

impl ToTokens for InputResultsValue<'_> {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let krate = krate();
        let results = self.0.iter().copied().map(InputResultValue);
        tokens.extend(quote! {
            #krate::InputResults::new(vec![
                #(#results),*
            ])
        });
    }
}

impl TryFrom<&ast::InputResult> for InputResult {
    type Error = Error;

    fn try_from(value: &ast::InputResult) -> Result<Self> {
        Ok(match value {
            ast::InputResult::Unchanged(_) => Self::Unchanged,
            ast::InputResult::Function(func) => Self::Function(func.try_into()?),
        })
    }
}

struct InputResultValue(InputResult);

impl ToTokens for InputResultValue {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let krate = krate();
        let variant = match self.0 {
            InputResult::Unchanged => quote!(Unchanged),
            InputResult::Function(f) => {
                let f = FunctionValue(f);
                quote!(Function(#f))
            }
        };
        tokens.extend(quote!(#krate::InputResult::#variant))
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
