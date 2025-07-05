use derive_more::{Deref, DerefMut};
use syn::{
    Error, Ident, LitBool, LitInt, Result, Token, Visibility, braced, bracketed, parenthesized,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    token::{Brace, Bracket, Paren},
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
    Nary(Punctuated<OperandType, Token![|]>),
    Tuples(Punctuated<OperandTuplesElement, Token![,]>),
}

impl Parse for Operands {
    fn parse(input: ParseStream) -> Result<Self> {
        let inner;
        bracketed!(inner in input);
        if inner.peek(Paren) || inner.peek(Token![...]) {
            Ok(Self::Tuples(Punctuated::parse_terminated(&inner)?))
        } else {
            Ok(Self::Nary(Punctuated::parse_terminated(&inner)?))
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

pub type Tuple<T> = Parenthesized<ParsePunctuated<T, Token![,]>>;

#[derive(Debug)]
pub struct OperandType {
    pub invert: Option<Token![!]>,
    pub name: BoolOrIdent,
    pub index: MaybeBracketed<LitInt>,
}

impl Parse for OperandType {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(Self {
            invert: input.parse()?,
            name: input.parse()?,
            index: input.parse()?,
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
    pub input_target_idx: MaybeBracketed<IntOrStar>,
    pub function: Function,
    pub input: Parenthesized<Ident>,
}

impl Parse for Operation {
    fn parse(input: ParseStream) -> Result<Self> {
        let inner;
        let name = input.parse()?;
        let eq = input.parse()?;
        let paren = parenthesized!(inner in input);
        let input_result_idx: MaybeBracketed<IntOrStar> = inner.parse()?;
        if input_result_idx.0.is_some() {
            inner.parse::<Token![:]>()?;
            inner.parse::<Token![=]>()?;
        }
        Ok(Self {
            name,
            eq,
            paren,
            input_target_idx: input_result_idx,
            function: inner.parse()?,
            input: inner.parse()?,
        })
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

#[derive(Debug)]
pub struct Bracketed<T> {
    pub bracket: Bracket,
    pub value: T,
}

impl<T> Parse for Bracketed<T>
where
    T: Parse,
{
    fn parse(input: ParseStream) -> Result<Self> {
        let inner;
        Ok(Self {
            bracket: bracketed!(inner in input),
            value: inner.parse()?,
        })
    }
}

#[derive(Debug)]
pub struct MaybeBracketed<T>(pub Option<Bracketed<T>>);

impl<T> Parse for MaybeBracketed<T>
where
    T: Parse,
{
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(Self(if input.peek(Bracket) {
            Some(input.parse()?)
        } else {
            None
        }))
    }
}

#[derive(Debug)]
pub struct Parenthesized<T> {
    pub paren: Paren,
    pub value: T,
}

impl<T> Parse for Parenthesized<T>
where
    T: Parse,
{
    fn parse(input: ParseStream) -> Result<Self> {
        let inner;
        Ok(Self {
            paren: parenthesized!(inner in input),
            value: inner.parse()?,
        })
    }
}

#[derive(Debug, Deref, DerefMut)]
pub struct ParsePunctuated<T, P>(Punctuated<T, P>);

impl<T, P> Parse for ParsePunctuated<T, P>
where
    T: Parse,
    P: Parse,
{
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(Self(Punctuated::parse_terminated(input)?))
    }
}

#[derive(Debug)]
#[allow(dead_code)]
pub enum IntOrStar {
    Int(LitInt),
    Star(Token![*]),
}

impl Parse for IntOrStar {
    fn parse(input: ParseStream) -> Result<Self> {
        if let Ok(int) = input.parse() {
            Ok(Self::Int(int))
        } else if let Ok(star) = input.parse() {
            Ok(Self::Star(star))
        } else {
            Err(Error::new(input.span(), "expected '*' or integer literal"))
        }
    }
}
