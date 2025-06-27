use syn::{
    Error, Ident, LitBool, LitInt, Result, Token, Visibility, braced, bracketed, parenthesized,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    token::{Brace, Bracket, Comma, Paren},
};

#[derive(Debug)]
pub struct Architecture {
    pub vis: Visibility,
    pub name: Ident,
    #[expect(unused)]
    pub brace: Brace,
    pub inner: ArchitectureInner,
}

impl Parse for Architecture {
    fn parse(input: ParseStream) -> Result<Self> {
        let inner;
        Ok(Architecture {
            vis: input.parse()?,
            name: input.parse()?,
            brace: braced!(inner in input),
            inner: inner.parse()?,
        })
    }
}

#[derive(Debug)]
pub struct ArchitectureInner {
    pub cells: Tuple<CellDef>,
    pub operands: Tuple<NameAndOperands>,
    pub operations: Tuple<Operation>,
    pub output: Option<Tuple<Ident>>,
}

impl Parse for ArchitectureInner {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut cells = None;
        let mut operands = None;
        let mut operations = None;
        let mut output = None;
        while !input.is_empty() {
            let ident: Ident = input.parse()?;
            match ident.to_string().as_str() {
                "cells" => cells = Some(input.parse()?),
                "operands" => operands = Some(input.parse()?),
                "operations" => operations = Some(input.parse()?),
                "output" => output = Some(input.parse()?),
                _ => return Err(Error::new(ident.span(), "invalid property")),
            }
            if !input.is_empty() {
                input.parse::<Token![,]>()?;
            }
        }
        match (cells, operands, operations) {
            (Some(cells), Some(operands), Some(operations)) => Ok(Self {
                cells,
                operands,
                operations,
                output,
            }),
            _ => Err(Error::new(input.span(), "missing property")),
        }
    }
}

#[derive(Debug)]
pub struct CellDef {
    #[expect(unused)]
    pub bracket: Bracket,
    pub name: Ident,
    pub num: Option<LitInt>,
}

impl Parse for CellDef {
    fn parse(input: ParseStream) -> Result<Self> {
        let inner;
        Ok(CellDef {
            bracket: bracketed!(inner in input),
            name: inner.parse()?,
            num: inner
                .parse::<Option<Token![;]>>()?
                .map(|_| inner.parse())
                .transpose()?,
        })
    }
}

#[derive(Debug)]
pub struct NameAndOperands {
    pub name: Ident,
    #[expect(unused)]
    pub eq: Token![=],
    pub operands: Operands,
}

impl Parse for NameAndOperands {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(Self {
            name: input.parse()?,
            eq: input.parse()?,
            operands: input.parse()?,
        })
    }
}

#[derive(Debug)]
pub enum Operands {
    Nary(OperandType),
    Tuples(Punctuated<OperandTuplesElement, Token![,]>),
}

impl Parse for Operands {
    fn parse(input: ParseStream) -> Result<Self> {
        let inner;
        bracketed!(inner in input);
        if inner.peek(Paren) || inner.peek(Token![...]) {
            Ok(Self::Tuples(Punctuated::parse_terminated(&inner)?))
        } else {
            Ok(Self::Nary(inner.parse()?))
        }
    }
}

#[derive(Debug)]
pub enum OperandTuplesElement {
    Tuple(Tuple<OperandType>),
    Ref(Ident),
}

impl Parse for OperandTuplesElement {
    fn parse(input: ParseStream) -> Result<Self> {
        if input.peek(Paren) {
            Ok(Self::Tuple(input.parse()?))
        } else {
            input.parse::<Token![...]>()?;
            Ok(Self::Ref(input.parse()?))
        }
    }
}

#[derive(Debug)]
pub struct Tuple<T> {
    pub paren: Paren,
    pub elements: Punctuated<T, Comma>,
}

impl<T> Parse for Tuple<T>
where
    T: Parse,
{
    fn parse(input: ParseStream) -> Result<Self> {
        let inner;
        Ok(Self {
            paren: parenthesized!(inner in input),
            elements: Punctuated::parse_terminated(&inner)?,
        })
    }
}

#[derive(Debug)]
pub struct OperandType {
    pub invert: Option<Token![!]>,
    pub name: BoolOrIdent,
    pub index: Option<(Bracket, LitInt)>,
}

impl Parse for OperandType {
    fn parse(input: ParseStream) -> Result<Self> {
        let invert = input.parse()?;
        let name = input.parse()?;
        let index = if input.peek(Bracket) {
            let inner;
            let bracket = bracketed!(inner in input);
            Some((bracket, inner.parse()?))
        } else {
            None
        };
        Ok(Self {
            invert,
            name,
            index,
        })
    }
}

#[derive(Debug)]
pub struct Operation {
    pub name: Ident,
    #[expect(unused)]
    pub eq: Token![=],
    #[expect(unused)]
    pub paren: Paren,
    pub input: Ident,
    pub input_results: Option<InputResults>,
    pub output_function: Option<Function>,
}

impl Parse for Operation {
    fn parse(input: ParseStream) -> Result<Self> {
        let inner;
        Ok(Self {
            name: input.parse()?,
            eq: input.parse()?,
            paren: parenthesized!(inner in input),
            input: inner.parse()?,
            input_results: inner
                .parse::<Option<Token![:]>>()?
                .map(|_| {
                    inner.parse::<Option<Token![=]>>()?;
                    inner.parse()
                })
                .transpose()?,
            output_function: inner
                .parse::<Option<Token![->]>>()?
                .map(|_| inner.parse())
                .transpose()?,
        })
    }
}

#[derive(Debug)]
pub enum InputResults {
    All(InputResult),
    Tuple(Tuple<InputResult>),
}

impl Parse for InputResults {
    fn parse(input: ParseStream) -> Result<Self> {
        if input.peek(Paren) {
            Ok(Self::Tuple(input.parse()?))
        } else {
            Ok(Self::All(input.parse()?))
        }
    }
}

#[derive(Debug)]
pub enum InputResult {
    Unchanged(Token![_]),
    Function(Function),
}

impl Parse for InputResult {
    fn parse(input: ParseStream) -> Result<Self> {
        if input.peek(Token![_]) {
            Ok(Self::Unchanged(input.parse()?))
        } else {
            Ok(Self::Function(input.parse()?))
        }
    }
}

#[derive(Debug)]
pub struct Function {
    pub inverted: Option<Token![!]>,
    pub gate: BoolOrIdent,
}

impl Parse for Function {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(Self {
            inverted: input.parse()?,
            gate: input.parse()?,
        })
    }
}

#[derive(Debug)]
pub enum BoolOrIdent {
    Bool(LitBool),
    Ident(Ident),
}

impl Parse for BoolOrIdent {
    fn parse(input: ParseStream) -> Result<Self> {
        if input.peek(LitBool) {
            Ok(Self::Bool(input.parse()?))
        } else {
            Ok(Self::Ident(input.parse()?))
        }
    }
}
