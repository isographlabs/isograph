use prelude::Postfix;
use serde::{Deserialize, Serialize};

use crate::{EntityNameAndSelectableName, Location, LocationFreeDiagnostic};

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

    pub fn printable<'a>(&'a self, print_location: PrintLocationFn<'a>) -> PrintableDiagnostic<'a> {
        PrintableDiagnostic {
            diagnostic: self,
            print_location,
        }
    }
}

pub type PrintLocationFn<'a> =
    Box<dyn Fn(Location, &mut std::fmt::Formatter<'_>) -> std::fmt::Result + 'a>;

pub fn noop_print_location_fn<'a>() -> PrintLocationFn<'a> {
    noop_print_location_fn_inner.boxed()
}

fn noop_print_location_fn_inner(
    _loc: Location,
    _formatter: &mut std::fmt::Formatter<'_>,
) -> std::fmt::Result {
    Ok(())
}

pub struct PrintableDiagnostic<'a> {
    diagnostic: &'a Diagnostic,
    print_location: PrintLocationFn<'a>,
}

impl std::fmt::Display for PrintableDiagnostic<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.diagnostic.0.message)?;
        if let Some(location) = self.diagnostic.location() {
            writeln!(f)?;
            (self.print_location)(location, f)?;
        }
        Ok(())
    }
}

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

pub type WithNonFatalDiagnostics<T> = WithGenericNonFatalDiagnostics<T, Diagnostic>;

#[derive(Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct WithGenericNonFatalDiagnostics<T, TError> {
    pub non_fatal_diagnostics: Vec<TError>,
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

impl<T: Default, TError> Default for WithGenericNonFatalDiagnostics<T, TError> {
    fn default() -> Self {
        Self {
            non_fatal_diagnostics: Default::default(),
            item: Default::default(),
        }
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Copy, Serialize, Deserialize)]
pub enum IsographCodeAction {
    CreateNewScalarSelectable(EntityNameAndSelectableName),
    CreateNewObjectSelectable(EntityNameAndSelectableName),
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
