use span::WithSpan;
use thiserror::Error;

use isograph_parser::ParseError;

pub trait HostLanguage: Sized {
    type Error: std::fmt::Display + std::error::Error;
    type LiteralContext;

    fn extract_iso_literals<'a>(&self, source: &'a str) -> ExtractedIsoLiterals<'a, Self>;
}

pub type ExtractedIsoLiterals<'a, THostLanguage> = Vec<
    WithErrors<
        WithSpan<(&'a str, <THostLanguage as HostLanguage>::LiteralContext)>,
        Vec<WithSpan<IsoLiteralError<THostLanguage>>>,
    >,
>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WithErrors<T, E> {
    pub item: T,
    pub errors: E,
}

#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum IsoLiteralError<THostLanguage: HostLanguage> {
    #[error("{0}")]
    Host(THostLanguage::Error),
    #[error("{0}")]
    Parse(ParseError),
}
