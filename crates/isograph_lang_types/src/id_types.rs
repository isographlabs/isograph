use u32_newtypes::{u32_conversion, u32_newtype};

// Any field defined on the server
u32_newtype!(ServerFieldId);
// A field that acts as an id
u32_newtype!(ServerStrongIdFieldId);

u32_conversion!(from: ServerStrongIdFieldId, to: ServerFieldId);

u32_newtype!(ClientFieldId);

u32_newtype!(ClientPointerId);

u32_newtype!(ServerObjectId);

u32_newtype!(ServerScalarId);

pub type SelectableServerFieldId = SelectionType<ServerScalarId, ServerObjectId>;

#[derive(Debug, Clone, Copy, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub enum SelectionType<TScalar, TObject> {
    Scalar(TScalar),
    Object(TObject),
}

impl TryFrom<SelectableServerFieldId> for ServerScalarId {
    type Error = ();

    fn try_from(value: SelectableServerFieldId) -> Result<Self, Self::Error> {
        match value {
            SelectableServerFieldId::Object(_) => Err(()),
            SelectableServerFieldId::Scalar(scalar_id) => Ok(scalar_id),
        }
    }
}

impl TryFrom<SelectableServerFieldId> for ServerObjectId {
    type Error = ();

    fn try_from(value: SelectableServerFieldId) -> Result<Self, Self::Error> {
        match value {
            SelectableServerFieldId::Object(object_id) => Ok(object_id),
            SelectableServerFieldId::Scalar(_) => Err(()),
        }
    }
}

u32_newtype!(RefetchQueryIndex);

/// Distinguishes between server-defined fields and locally-defined fields.
/// TFieldAssociatedData can be a ScalarFieldName in an unvalidated schema, or a
/// ScalarId, in a validated schema.
///
/// TLocalType can be an UnvalidatedTypeName in an unvalidated schema, or an
/// DefinedTypeId in a validated schema.
///
/// Note that locally-defined fields do **not** only include fields defined in
/// an iso field literal. Refetch fields and generated mutation fields are
/// also local fields.
#[derive(Debug, Clone, Copy, Ord, PartialOrd, PartialEq, Eq)]
pub enum DefinitionLocation<TServer, TClient> {
    Server(TServer),
    Client(TClient),
}
