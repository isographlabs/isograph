pub trait HasName {
    type Name;
    fn name(&self) -> Self::Name;
}

/// Distinguishes between server fields and locally-defined resolver fields.
/// TFieldAssociatedType can be a ScalarFieldName in an unvalidated schema, or a
/// ScalarId, in a validated schema.
///
/// TResolverType can be an UnvalidatedTypeName in an unvalidated schema, or an
/// OutputTypeId in a validated schema.
#[derive(Debug, Clone, Copy)]
pub enum DefinedField<TFieldAssociatedType, TResolverType> {
    ServerField(TFieldAssociatedType),
    ResolverField(TResolverType),
}

impl<TFieldAssociatedType, TResolverType> DefinedField<TFieldAssociatedType, TResolverType> {
    pub fn as_server_field(&self) -> Option<&TFieldAssociatedType> {
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
