pub trait HasName {
    type Name;
    fn name(&self) -> Self::Name;
}

/// A trait that constrains the types that are valid in a TypeAnnotation.
/// TypeNames and TypeIds are valid types for a TypeAnnotation.
/// TODO this trait should be removed
pub trait ValidTypeAnnotationInnerType {}

/// Distinguishes between server fields and locally-defined resolver fields.
/// TServerType can be a ScalarFieldName in an unvalidated schema, or a
/// ScalarId, in a validated schema.
///
/// TResolverType can be an UnvalidatedTypeName in an unvalidated schema, or an
/// OutputTypeId in a validated schema.
#[derive(Debug, Clone, Copy)]
pub enum DefinedField<TServerType: ValidTypeAnnotationInnerType, TResolverType> {
    ServerField(TServerType),
    ResolverField(TResolverType), // Resolvers have an opaque scalar type
}

impl<TServerType: ValidTypeAnnotationInnerType, TResolverType>
    DefinedField<TServerType, TResolverType>
{
    pub fn as_server_field(&self) -> Option<&TServerType> {
        match self {
            DefinedField::ServerField(server_field) => Some(server_field),
            DefinedField::ResolverField(_) => None,
        }
    }

    pub fn as_resolver_field(&self) -> Option<&TResolverType> {
        match self {
            DefinedField::ServerField(_) => None,
            DefinedField::ResolverField(resolver_field) => Some(resolver_field),
        }
    }
}
