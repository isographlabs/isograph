use span::WithSpan;
use thiserror::Error;

use isograph_parser::ParseError;

pub trait HostLanguage: Sized {
    type Error: std::fmt::Display + std::error::Error;
    type LiteralContext;

    fn extract_iso_literals<'a>(
        &self,
        source: &'a str,
    ) -> Vec<WithSpan<(&'a str, Self::LiteralContext)>>;
}

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
