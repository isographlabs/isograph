use std::fmt::Debug;

/// A trait that encapsulates all the types over which a schema, fields, etc.
/// are generic. As we go from parsed -> various states of validated -> fully
/// validated, we will get objects that are generic over a different type
/// that implements SchemaValidationState.
pub trait SchemaValidationState: Debug {
    /// The associated data type of scalars in client fields' selection sets and unwraps
    /// - Unvalidated: ()
    /// - Validated: ValidatedFieldDefinitionLocation
    ///   i.e. DefinedField<ServerFieldId, ClientFieldId>
    type SelectionTypeSelectionScalarFieldAssociatedData: Debug;

    /// The associated data type of linked fields in client fields' selection sets and unwraps
    /// - Unvalidated: ()
    /// - Validated: ObjectId
    type SelectionTypeSelectionLinkedFieldAssociatedData: Debug;

    /// The associated data type of client fields' variable definitions
    /// - Unvalidated: UnvalidatedTypeName
    /// - Validated: FieldDefinition
    type VariableDefinitionInnerType: Debug + Clone + Ord;

    /// What we store in entrypoints
    /// - Unvalidated: (TextSource, WithSpan<ObjectTypeAndField>)
    /// - Validated: (ObjectId, ClientFieldId)
    type Entrypoint: Debug;
}
