use std::collections::{BTreeMap, HashMap};

use common_lang_types::{
    JavascriptName, Location, TextSource, UnvalidatedTypeName, WithLocation, WithSpan,
};
use graphql_lang_types::GraphQLTypeAnnotation;
use intern::string_key::Intern;
use isograph_lang_types::{
    ClientFieldId, EntrypointTypeAndField, IsographSelectionVariant, LinkedFieldSelection,
    SelectableServerFieldId, ServerFieldId, ServerScalarId, VariableDefinition,
};

use crate::{
    ClientField, FieldDefinitionLocation, Schema, SchemaScalar, SchemaServerField,
    SchemaValidationState, ServerFieldData, UseRefetchFieldRefetchStrategy,
};
use lazy_static::lazy_static;

lazy_static! {
    pub static ref STRING_JAVASCRIPT_TYPE: JavascriptName = "string".intern().into();
}

#[derive(Debug)]
pub struct UnvalidatedSchemaState {}

impl SchemaValidationState for UnvalidatedSchemaState {
    type ServerFieldTypeAssociatedData = GraphQLTypeAnnotation<UnvalidatedTypeName>;
    type ClientFieldSelectionScalarFieldAssociatedData = IsographSelectionVariant;
    type ClientFieldSelectionLinkedFieldAssociatedData = IsographSelectionVariant;
    type VariableDefinitionInnerType = UnvalidatedTypeName;
    type Entrypoint = Vec<(TextSource, WithSpan<EntrypointTypeAndField>)>;
}

pub type UnvalidatedSchema = Schema<UnvalidatedSchemaState>;

/// On unvalidated schema objects, the encountered types are either a type annotation
/// for server fields with an unvalidated inner type, or a ScalarFieldName (the name of the
/// client field.)
pub type UnvalidatedObjectFieldInfo = FieldDefinitionLocation<ServerFieldId, ClientFieldId>;

pub(crate) type UnvalidatedSchemaSchemaField = SchemaServerField<
    <UnvalidatedSchemaState as SchemaValidationState>::ServerFieldTypeAssociatedData,
    <UnvalidatedSchemaState as SchemaValidationState>::VariableDefinitionInnerType,
>;

pub type UnvalidatedVariableDefinition = VariableDefinition<
    <UnvalidatedSchemaState as SchemaValidationState>::VariableDefinitionInnerType,
>;

pub type UnvalidatedClientField = ClientField<
    <UnvalidatedSchemaState as SchemaValidationState>::ClientFieldSelectionScalarFieldAssociatedData,
    <UnvalidatedSchemaState as SchemaValidationState>::ClientFieldSelectionLinkedFieldAssociatedData,
    <UnvalidatedSchemaState as SchemaValidationState>::VariableDefinitionInnerType,
>;

pub type UnvalidatedLinkedFieldSelection = LinkedFieldSelection<
    <UnvalidatedSchemaState as SchemaValidationState>::ClientFieldSelectionScalarFieldAssociatedData,
    <UnvalidatedSchemaState as SchemaValidationState>::ClientFieldSelectionLinkedFieldAssociatedData,
>;

pub type UnvalidatedRefetchFieldStrategy = UseRefetchFieldRefetchStrategy<
    <UnvalidatedSchemaState as SchemaValidationState>::ClientFieldSelectionScalarFieldAssociatedData,
    <UnvalidatedSchemaState as SchemaValidationState>::ClientFieldSelectionLinkedFieldAssociatedData,
>;

impl Default for UnvalidatedSchema {
    fn default() -> Self {
        Self::new()
    }
}

impl UnvalidatedSchema {
    pub fn new() -> Self {
        // TODO add __typename
        let fields = vec![];
        let client_fields = vec![];
        let objects = vec![];
        let mut scalars = vec![];
        let mut defined_types = HashMap::default();

        let id_type_id = add_schema_defined_scalar_type(
            &mut scalars,
            &mut defined_types,
            "ID",
            *STRING_JAVASCRIPT_TYPE,
        );
        let string_type_id = add_schema_defined_scalar_type(
            &mut scalars,
            &mut defined_types,
            "String",
            *STRING_JAVASCRIPT_TYPE,
        );
        let boolean_type_id = add_schema_defined_scalar_type(
            &mut scalars,
            &mut defined_types,
            "Boolean",
            "boolean".intern().into(),
        );
        let float_type_id = add_schema_defined_scalar_type(
            &mut scalars,
            &mut defined_types,
            "Float",
            "number".intern().into(),
        );
        let int_type_id = add_schema_defined_scalar_type(
            &mut scalars,
            &mut defined_types,
            "Int",
            "number".intern().into(),
        );
        let null_type_id = add_schema_defined_scalar_type(
            &mut scalars,
            &mut defined_types,
            // The Null type should never be printed, at least for GraphQL.
            // TODO we should make this an Option and emit an error (or less
            // ideally, panic) if this is printed.
            "NullDoesNotExistIfThisIsPrintedThisIsABug",
            "number".intern().into(),
        );

        Self {
            server_fields: fields,
            client_fields,
            entrypoints: Default::default(),
            server_field_data: ServerFieldData {
                server_objects: objects,
                server_scalars: scalars,
                defined_types,
            },

            id_type_id,
            string_type_id,
            int_type_id,
            float_type_id,
            boolean_type_id,
            null_type_id,

            fetchable_types: BTreeMap::new(),
        }
    }
}

fn add_schema_defined_scalar_type(
    scalars: &mut Vec<SchemaScalar>,
    defined_types: &mut HashMap<UnvalidatedTypeName, SelectableServerFieldId>,
    field_name: &'static str,
    javascript_name: JavascriptName,
) -> ServerScalarId {
    let scalar_id = scalars.len().into();

    // TODO this is problematic, we have no span (or really, no location) associated with this
    // schema-defined scalar, so we will not be able to properly show error messages if users
    // e.g. have Foo implements String
    let typename = WithLocation::new(field_name.intern().into(), Location::generated());
    scalars.push(SchemaScalar {
        description: None,
        name: typename,
        id: scalar_id,
        javascript_name,
    });
    defined_types.insert(
        typename.item.into(),
        SelectableServerFieldId::Scalar(scalar_id),
    );
    scalar_id
}
