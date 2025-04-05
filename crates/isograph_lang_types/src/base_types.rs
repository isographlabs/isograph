// These must be kept in-sync with `impl_base_types` or things will not compile!

use std::fmt::Display;

use common_lang_types::{SelectableName, UnvalidatedTypeName};
use intern::Lookup;

/// Distinguishes between server-defined items and locally-defined items.
///
/// Examples include:
///
/// - server fields vs client fields.
/// - schema server fields (objects) vs client pointers
#[derive(Debug, Clone, Copy, Ord, PartialOrd, PartialEq, Eq)]
pub enum DefinitionLocation<TServer, TClient> {
    Server(TServer),
    Client(TClient),
}

impl<TServer, TClient> DefinitionLocation<TServer, TClient> {
    pub fn as_server(&self) -> Option<&TServer> {
        match self {
            DefinitionLocation::Server(s) => Some(s),
            DefinitionLocation::Client(_) => None,
        }
    }

    pub fn as_server_result(&self) -> Result<&TServer, &TClient> {
        match self {
            DefinitionLocation::Server(s) => Ok(s),
            DefinitionLocation::Client(c) => Err(c),
        }
    }

    pub fn as_client(&self) -> Option<&TClient> {
        match self {
            DefinitionLocation::Server(_) => None,
            DefinitionLocation::Client(c) => Some(c),
        }
    }

    pub fn as_client_result(&self) -> Result<&TClient, &TServer> {
        match self {
            DefinitionLocation::Server(s) => Err(s),
            DefinitionLocation::Client(c) => Ok(c),
        }
    }
}

/// Distinguishes between items are are "scalar-like" and objects that
/// are "object-like". Examples include:
///
/// - client fields vs client pointers
/// - scalar field selections (i.e. those without selection sets) vs
///   linked field selections.
/// - schema scalars vs schema objects
#[derive(Debug, Clone, Copy, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub enum SelectionType<TScalar, TObject> {
    Scalar(TScalar),
    Object(TObject),
}

// For traits that we define, we can use crates in the impl_base_traits crate.
// For others, we implement them manually. This can be fixed!
impl<T0: Lookup, T1: Lookup> Lookup for SelectionType<T0, T1> {
    fn lookup(self) -> &'static str {
        match self {
            SelectionType::Scalar(s) => s.lookup(),
            SelectionType::Object(o) => o.lookup(),
        }
    }
}

impl<T0: Display, T1: Display> Display for SelectionType<T0, T1> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SelectionType::Scalar(s) => s.fmt(f),
            SelectionType::Object(o) => o.fmt(f),
        }
    }
}

impl<T0: Into<SelectableName>, T1: Into<SelectableName>> From<SelectionType<T0, T1>>
    for SelectableName
{
    fn from(value: SelectionType<T0, T1>) -> Self {
        match value {
            SelectionType::Scalar(s) => s.into(),
            SelectionType::Object(o) => o.into(),
        }
    }
}

impl<T0: Into<UnvalidatedTypeName>, T1: Into<UnvalidatedTypeName>> From<SelectionType<T0, T1>>
    for UnvalidatedTypeName
{
    fn from(value: SelectionType<T0, T1>) -> Self {
        match value {
            SelectionType::Scalar(s) => s.into(),
            SelectionType::Object(o) => o.into(),
        }
    }
}

impl<TScalar, TObject> SelectionType<TScalar, TObject> {
    pub fn as_scalar(&self) -> Option<&TScalar> {
        match self {
            SelectionType::Scalar(s) => Some(s),
            SelectionType::Object(_) => None,
        }
    }

    pub fn as_scalar_result(&self) -> Result<&TScalar, &TObject> {
        match self {
            SelectionType::Scalar(s) => Ok(s),
            SelectionType::Object(o) => Err(o),
        }
    }

    pub fn as_object(&self) -> Option<&TObject> {
        match self {
            SelectionType::Scalar(_) => None,
            SelectionType::Object(o) => Some(o),
        }
    }

    pub fn as_object_result(&self) -> Result<&TObject, &TScalar> {
        match self {
            SelectionType::Scalar(s) => Err(s),
            SelectionType::Object(o) => Ok(o),
        }
    }
}
