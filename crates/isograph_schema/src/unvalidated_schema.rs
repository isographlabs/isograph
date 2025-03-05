use std::collections::{BTreeMap, HashMap};

use common_lang_types::{
    IsographObjectTypeName, JavascriptName, Location, TextSource, UnvalidatedTypeName,
    WithLocation, WithSpan,
};
use intern::string_key::Intern;
use isograph_lang_types::{
    DefinitionLocation, EntrypointDeclaration, LinkedFieldSelection, ObjectSelectionDirectiveSet,
    ScalarSelectionDirectiveSet, SelectableServerFieldId, ServerFieldId, ServerScalarId,
    VariableDefinition,
};

use crate::{
    schema_validation_state::SchemaValidationState, ClientField, ClientFieldOrPointerId,
    ClientPointer, OutputFormat, Schema, SchemaScalar, SchemaServerField, ServerFieldData,
    UseRefetchFieldRefetchStrategy, ValidatedSelection,
};
use lazy_static::lazy_static;

lazy_static! {
    pub static ref STRING_JAVASCRIPT_TYPE: JavascriptName = "string".intern().into();
}

#[derive(Debug)]
pub struct UnvalidatedSchemaState {}

impl SchemaValidationState for UnvalidatedSchemaState {
    type SelectionTypeSelectionScalarFieldAssociatedData = ScalarSelectionDirectiveSet;
    type SelectionTypeSelectionLinkedFieldAssociatedData = ObjectSelectionDirectiveSet;
    type VariableDefinitionInnerType = UnvalidatedTypeName;
    type Entrypoint = Vec<(TextSource, WithSpan<EntrypointDeclaration>)>;
}

#[derive(Debug, Clone)]
pub enum SchemaServerLinkedFieldFieldVariant {
    LinkedField,
    InlineFragment(ServerFieldTypeAssociatedDataInlineFragment),
}

#[derive(Debug, Clone)]
pub struct ServerFieldTypeAssociatedDataInlineFragment {
    pub server_field_id: ServerFieldId,
    pub concrete_type: IsographObjectTypeName,
    pub reader_selection_set: Vec<WithSpan<ValidatedSelection>>,
}

pub type UnvalidatedSchema<TOutputFormat> = Schema<UnvalidatedSchemaState, TOutputFormat>;

/// On unvalidated schema objects, the encountered types are either a type annotation
/// for server fields with an unvalidated inner type, or a ScalarFieldName (the name of the
/// client field.)
pub type UnvalidatedObjectFieldInfo = DefinitionLocation<ServerFieldId, ClientFieldOrPointerId>;

pub type UnvalidatedSchemaSchemaField<TOutputFormat> = SchemaServerField<
    <UnvalidatedSchemaState as SchemaValidationState>::VariableDefinitionInnerType,
    TOutputFormat,
>;

pub type UnvalidatedVariableDefinition = VariableDefinition<
    <UnvalidatedSchemaState as SchemaValidationState>::VariableDefinitionInnerType,
>;

pub type UnvalidatedClientField<TOutputFormat> = ClientField<
    <UnvalidatedSchemaState as SchemaValidationState>::SelectionTypeSelectionScalarFieldAssociatedData,
    <UnvalidatedSchemaState as SchemaValidationState>::SelectionTypeSelectionLinkedFieldAssociatedData,
    <UnvalidatedSchemaState as SchemaValidationState>::VariableDefinitionInnerType,
    TOutputFormat,
>;

pub type UnvalidatedClientPointer<TOutputFormat> = ClientPointer<
    <UnvalidatedSchemaState as SchemaValidationState>::SelectionTypeSelectionScalarFieldAssociatedData,
    <UnvalidatedSchemaState as SchemaValidationState>::SelectionTypeSelectionLinkedFieldAssociatedData,
    <UnvalidatedSchemaState as SchemaValidationState>::VariableDefinitionInnerType,
    TOutputFormat,
>;

pub type UnvalidatedLinkedFieldSelection = LinkedFieldSelection<
    <UnvalidatedSchemaState as SchemaValidationState>::SelectionTypeSelectionScalarFieldAssociatedData,
    <UnvalidatedSchemaState as SchemaValidationState>::SelectionTypeSelectionLinkedFieldAssociatedData,
>;

pub type UnvalidatedRefetchFieldStrategy = UseRefetchFieldRefetchStrategy<
    <UnvalidatedSchemaState as SchemaValidationState>::SelectionTypeSelectionScalarFieldAssociatedData,
    <UnvalidatedSchemaState as SchemaValidationState>::SelectionTypeSelectionLinkedFieldAssociatedData,
>;

impl<TOutputFormat: OutputFormat> Default for UnvalidatedSchema<TOutputFormat> {
    fn default() -> Self {
        Self::new()
    }
}

impl<TOutputFormat: OutputFormat> UnvalidatedSchema<TOutputFormat> {
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
            client_types: client_fields,
            entrypoints: Default::default(),
            server_field_data: ServerFieldData {
                server_objects: objects,
                server_scalars: scalars,
                defined_types,

                id_type_id,
                string_type_id,
                int_type_id,
                float_type_id,
                boolean_type_id,
                null_type_id,
            },
            fetchable_types: BTreeMap::new(),
        }
    }
}

fn add_schema_defined_scalar_type<TOutputFormat: OutputFormat>(
    scalars: &mut Vec<SchemaScalar<TOutputFormat>>,
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
        output_format: std::marker::PhantomData,
    });
    defined_types.insert(
        typename.item.into(),
        SelectableServerFieldId::Scalar(scalar_id),
    );
    scalar_id
}
