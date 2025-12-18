use std::fmt::Display;

use prelude::Postfix;
use serde::{Deserialize, Serialize, de};

use crate::{Location, LocationFreeDiagnostic, ParentObjectEntityNameAndSelectableName};

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Diagnostic(pub Box<DiagnosticData>);

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct DiagnosticData {
    /// A human-readable message.
    pub message: String,

    /// The primary location where the message originated.
    pub location: Option<Location>,

    /// Any associated code actions
    pub code_actions: Vec<IsographCodeAction>,
}

impl Diagnostic {
    pub fn new(message: String, location: Option<Location>) -> Diagnostic {
        Diagnostic(
            DiagnosticData {
                message,
                location,
                code_actions: vec![],
            }
            .boxed(),
        )
    }

    pub fn location(&self) -> Option<Location> {
        self.0.location
    }

    pub fn new_with_code_actions(
        message: String,
        location: Option<Location>,
        code_actions: Vec<IsographCodeAction>,
    ) -> Diagnostic {
        Diagnostic(
            DiagnosticData {
                message,
                location,
                code_actions,
            }
            .boxed(),
        )
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

// We often have functions that return single Diagnostics. It's useful to be able to
// use ? on those in functions which return Result<T, Vec<Diagnostic>>
impl From<Diagnostic> for Vec<Diagnostic> {
    fn from(value: Diagnostic) -> Self {
        vec![value]
    }
}

#[derive(Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct WithNonFatalDiagnostics<T> {
    pub non_fatal_diagnostics: Vec<Diagnostic>,
    pub item: T,
}

impl<T> WithNonFatalDiagnostics<T> {
    pub fn new(item: T, non_fatal_diagnostics: Vec<Diagnostic>) -> Self {
        WithNonFatalDiagnostics {
            non_fatal_diagnostics,
            item,
        }
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Copy, Serialize, Deserialize)]
pub enum IsographCodeAction {
    CreateNewScalarSelectable(ParentObjectEntityNameAndSelectableName),
    CreateNewObjectSelectable(ParentObjectEntityNameAndSelectableName),
}

impl From<LocationFreeDiagnostic> for Diagnostic {
    fn from(value: LocationFreeDiagnostic) -> Self {
        Self(
            DiagnosticData {
                message: value.to_string(),
                location: None,
                code_actions: vec![],
            }
            .boxed(),
        )
    }
}
