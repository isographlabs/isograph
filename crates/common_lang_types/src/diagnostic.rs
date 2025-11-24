use std::fmt::Display;

use prelude::Postfix;
use serde::de;

use crate::Location;

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Diagnostic(pub Box<DiagnosticData>);

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct DiagnosticData {
    /// A human-readable message.
    pub message: String,

    /// The primary location where the message originated.
    pub location: Option<Location>,
}

impl Diagnostic {
    pub fn new(message: String, location: Option<Location>) -> Diagnostic {
        Diagnostic(DiagnosticData { message, location }.boxed())
    }
}

impl Display for Diagnostic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.message)?;
        if let Some(location) = self.0.location {
            write!(f, "\n{}", location)?;
        }
        Ok(())
    }
}

impl de::Error for Diagnostic {
    fn custom<T>(msg: T) -> Self
    where
        T: core::fmt::Display,
    {
        Diagnostic::new(msg.to_string(), None)
    }
}

impl std::error::Error for Diagnostic {}

pub type DiagnosticResult<T> = Result<T, Diagnostic>;
// TODO we should not do this
pub type DiagnosticVecResult<T> = Result<T, Vec<Diagnostic>>;
