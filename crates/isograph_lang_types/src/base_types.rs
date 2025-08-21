// These must be kept in-sync with `impl_base_types` or things will not compile!

use std::fmt::Display;

use common_lang_types::{SelectableName, UnvalidatedTypeName};
use intern::Lookup;
use resolve_position::ResolvePosition;

use crate::{ScalarSelection, UnvalidatedSelection};

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

    pub fn variant_name(&self) -> &'static str {
        match self {
            DefinitionLocation::Server(_) => "Server",
            DefinitionLocation::Client(_) => "Client",
        }
    }
}

impl<TServerScalar, TServerObject, TClientScalar, TClientObject>
    DefinitionLocation<
        SelectionType<TServerScalar, TServerObject>,
        SelectionType<TClientScalar, TClientObject>,
    >
{
    pub fn as_scalar(self) -> Option<DefinitionLocation<TServerScalar, TClientScalar>> {
        match self {
            DefinitionLocation::Server(server) => {
                Some(DefinitionLocation::Server(server.as_scalar()?))
            }
            DefinitionLocation::Client(client) => {
                Some(DefinitionLocation::Client(client.as_scalar()?))
            }
        }
    }

    pub fn as_object(self) -> Option<DefinitionLocation<TServerObject, TClientObject>> {
        match self {
            DefinitionLocation::Server(server) => {
                Some(DefinitionLocation::Server(server.as_object()?))
            }
            DefinitionLocation::Client(client) => {
                Some(DefinitionLocation::Client(client.as_object()?))
            }
        }
    }
}

impl<TServerObject, TServerScalar, TClientObject, TClientScalar>
    DefinitionLocation<
        SelectionType<TServerScalar, TServerObject>,
        SelectionType<TClientScalar, TClientObject>,
    >
{
    pub fn transpose(
        &self,
    ) -> SelectionType<
        DefinitionLocation<&TServerScalar, &TClientScalar>,
        DefinitionLocation<&TServerObject, &TClientObject>,
    > {
        match self {
            DefinitionLocation::Server(SelectionType::Object(object)) => {
                SelectionType::Object(DefinitionLocation::Server(object))
            }
            DefinitionLocation::Server(SelectionType::Scalar(scalar)) => {
                SelectionType::Scalar(DefinitionLocation::Server(scalar))
            }
            DefinitionLocation::Client(SelectionType::Object(object)) => {
                SelectionType::Object(DefinitionLocation::Client(object))
            }
            DefinitionLocation::Client(SelectionType::Scalar(scalar)) => {
                SelectionType::Scalar(DefinitionLocation::Client(scalar))
            }
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
    pub fn as_ref(&self) -> SelectionType<&TScalar, &TObject> {
        match self {
            SelectionType::Scalar(s) => SelectionType::Scalar(s),
            SelectionType::Object(o) => SelectionType::Object(o),
        }
    }

    pub fn as_scalar(self) -> Option<TScalar> {
        match self {
            SelectionType::Scalar(s) => Some(s),
            SelectionType::Object(_) => None,
        }
    }

    pub fn as_scalar_result(self) -> Result<TScalar, TObject> {
        match self {
            SelectionType::Scalar(s) => Ok(s),
            SelectionType::Object(o) => Err(o),
        }
    }

    pub fn as_object(self) -> Option<TObject> {
        match self {
            SelectionType::Scalar(_) => None,
            SelectionType::Object(o) => Some(o),
        }
    }

    pub fn as_object_result(self) -> Result<TObject, TScalar> {
        match self {
            SelectionType::Scalar(s) => Err(s),
            SelectionType::Object(o) => Ok(o),
        }
    }
}

// A blanket impl for SelectionType for ResolvedNode. Note that this will not work
// in all circumstances, but because it requires that the Parent associated type
// for both TScalar and TObject are the same. That will probably usually be the case,
// but it's not guaranteed. For example, if an entrypoint declaration
// `entrypoint Query.Foo` treated `Foo` as a scalar field selection (i.e. objects were
// disallowed there), then ScalarFieldSelection's Parent would be a larger enum than
// ObjectFieldSelection.
//
// That's not the case right now, but it may come up. And in that case, we can
// (probably) manually impl SelectionType for specific concrete types.
impl ResolvePosition for UnvalidatedSelection {
    type Parent<'a>
        = <ScalarSelection<()> as ResolvePosition>::Parent<'a>
    where
        Self: 'a;

    type ResolvedNode<'a>
        = <ScalarSelection<()> as ResolvePosition>::ResolvedNode<'a>
    where
        Self: 'a;

    fn resolve<'a>(
        &'a self,
        parent: Self::Parent<'a>,
        position: common_lang_types::Span,
    ) -> Self::ResolvedNode<'a> {
        match self {
            SelectionType::Scalar(scalar) => scalar.resolve(parent, position),
            SelectionType::Object(object) => object.resolve(parent, position),
        }
    }
}
