// These must be kept in-sync with `impl_base_types` or things will not compile!

use std::fmt::Display;

use common_lang_types::SelectableFieldName;
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

impl<T0: Into<SelectableFieldName>, T1: Into<SelectableFieldName>> From<SelectionType<T0, T1>>
    for SelectableFieldName
{
    fn from(value: SelectionType<T0, T1>) -> Self {
        match value {
            SelectionType::Scalar(s) => s.into(),
            SelectionType::Object(o) => o.into(),
        }
    }
}
