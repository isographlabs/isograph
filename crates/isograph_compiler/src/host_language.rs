use std::fmt::Debug;
use std::hash::Hash;
use std::path::PathBuf;

use span::WithSpan;
use thiserror::Error;

use isograph_parser::ParseError;

use crate::IsographState;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IsoLiteralExtraction<THostLanguage: HostLanguage> {
    pub iso_literal_text: String,
    pub iso_literal_start_index: usize,
    pub context: THostLanguage::LiteralContext,
}

pub trait HostLanguage:
    Copy + Clone + Debug + Eq + PartialEq + Ord + PartialOrd + Hash + Send + Sync + Sized + 'static
{
    type Error: std::fmt::Display + std::error::Error + Clone + PartialEq + Eq + 'static;
    type LiteralContext: Clone + PartialEq + Eq + Debug + 'static;

    fn extract_iso_literals_from_source(
        source: &str,
    ) -> Vec<WithSpan<(&str, Self::LiteralContext)>>;

    fn extract_iso_literals(
        db: &IsographState<Self>,
        path: PathBuf,
    ) -> &Option<Vec<IsoLiteralExtraction<Self>>>;
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
