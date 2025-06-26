use syn::{
    Ident, LitInt, Token, Visibility, braced, bracketed, parenthesized,
    parse::{self, Parse},
    punctuated::Punctuated,
    token::{Brace, Bracket},
};

#[derive(Debug)]
pub struct Architecture {
    pub vis: Visibility,
    pub name: Ident,
    pub brace: Brace,
    pub inner: ArchitectureInner,
}

impl Parse for Architecture {
    fn parse(input: parse::ParseStream) -> syn::Result<Self> {
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
    pub cells: Punctuated<CellDef, Token![,]>,
}

impl Parse for ArchitectureInner {
    fn parse(input: parse::ParseStream) -> syn::Result<Self> {
        let mut cells = None;
        while !input.is_empty() {
            let ident: Ident = input.parse()?;
            let inner;
            parenthesized!(inner in input);
            match ident.to_string().as_str() {
                "cells" => cells = Some(Punctuated::parse_terminated(&inner)?),
                _ => return Err(syn::Error::new(ident.span(), "invalid property")),
            }
        }
        match (cells,) {
            (Some(cells),) => Ok(Self { cells }),
            _ => Err(syn::Error::new(input.span(), "missing property")),
        }
    }
}

#[derive(Debug)]
pub struct CellDef {
    pub bracket: Bracket,
    pub name: Ident,
    pub num: Option<LitInt>,
}

impl Parse for CellDef {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
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
