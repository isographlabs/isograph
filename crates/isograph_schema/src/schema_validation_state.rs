use std::fmt::Debug;

/// A trait that encapsulates all the types over which a schema, fields, etc.
/// are generic. As we go from parsed -> various states of validated -> fully
/// validated, we will get objects that are generic over a different type
/// that implements SchemaValidationState.
pub trait SchemaValidationState: Debug {
    /// A SchemaServerField contains a associated_data: TypeAnnotation<ServerFieldTypeAssociatedData>
    /// - Unvalidated: UnvalidatedTypeName
    /// - Validated: DefinedTypeId
    type ServerFieldTypeAssociatedData: Debug;

    /// The associated data type of scalars in client fields' selection sets and unwraps
    /// - Unvalidated: ()
    /// - Validated: ValidatedFieldDefinitionLocation
    ///   i.e. DefinedField<ServerFieldId, ClientFieldId>
    type ClientTypeSelectionScalarFieldAssociatedData: Debug;

    /// The associated data type of linked fields in client fields' selection sets and unwraps
    /// - Unvalidated: ()
    /// - Validated: ObjectId
    type ClientTypeSelectionLinkedFieldAssociatedData: Debug;

    /// The associated data type of client fields' variable definitions
    /// - Unvalidated: UnvalidatedTypeName
    /// - Validated: FieldDefinition
    type VariableDefinitionInnerType: Debug + Clone + Ord;

    /// What we store in entrypoints
    /// - Unvalidated: (TextSource, WithSpan<ObjectTypeAndField>)
    /// - Validated: (ObjectId, ClientFieldId)
    type Entrypoint: Debug;
}
