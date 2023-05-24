use crate::TypeWithoutFieldsId;

pub trait HasName {
    type Name;
    fn name(&self) -> Self::Name;
}

/// A trait that constrains the types that are valid in a TypeAnnotation.
/// TypeNames and TypeIds are valid types for a TypeAnnotation.
pub trait ValidTypeAnnotationInnerType {}

/// Distinguishes between server fields and locally-defined resolver fields.
/// TServerType can be a ScalarFieldName in an unvalidated schema, or a
/// TypeWithoutFieldsId, in a validated schema.
///
/// TResolverType can be an UnvalidatedTypeName in an unvalidated schema, or an
/// OutputTypeId in a validated schema.
#[derive(Debug, Clone, Copy)]
pub enum DefinedField<TServerType: ValidTypeAnnotationInnerType, TResolverType> {
    ServerField(TServerType),
    ResolverField(TResolverType), // Resolvers have an opaque scalar type
}
// TODO map both types

/// Used to constrain the valid types that a ScalarFieldSelection can have to
/// ScalarFieldName (for unvalidated scalar field selections) and DefinedFieldType<TypeWithoutFieldsId>
/// (for validated scalar field selections).
pub trait ValidScalarFieldType {}

impl ValidScalarFieldType for DefinedField<TypeWithoutFieldsId, ()> {}

// /// Used to constrain the valid types that a LinkedFieldSelection can have to
// /// LinkedFieldName (for unvalidated linked field selections) and TypeWithFieldsId
// /// (for validated linked field selections).
pub trait ValidLinkedFieldType {}
