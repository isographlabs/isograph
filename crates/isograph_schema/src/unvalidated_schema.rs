use std::collections::HashMap;

use common_lang_types::{
    JavascriptName, Location, TextSource, UnvalidatedTypeName, WithLocation, WithSpan,
};
use graphql_lang_types::TypeAnnotation;
use intern::string_key::Intern;
use isograph_lang_types::{
    ClientFieldId, EntrypointTypeAndField, LinkedFieldSelection, ScalarId, SelectableFieldId,
};

use crate::{
    ClientField, FieldDefinitionLocation, Schema, SchemaData, SchemaObject, SchemaScalar,
    SchemaServerField, SchemaValidationState,
};
use lazy_static::lazy_static;

lazy_static! {
    pub static ref STRING_JAVASCRIPT_TYPE: JavascriptName = "string".intern().into();
}

#[derive(Debug)]
pub struct UnvalidatedSchemaState {}

impl SchemaValidationState for UnvalidatedSchemaState {
    type FieldTypeAssociatedData = UnvalidatedTypeName;
    // N.B. this must be kept in sync with client_field_declaration.rs
    type ClientFieldSelectionScalarFieldAssociatedData = ();
    // N.B. this must be kept in sync with client_field_declaration.rs
    type ClientFieldSelectionLinkedFieldAssociatedData = ();
    type ClientFieldVariableDefinitionAssociatedData = UnvalidatedTypeName;
    type EncounteredField = UnvalidatedObjectFieldInfo;
    type Entrypoint = (TextSource, WithSpan<EntrypointTypeAndField>);
}

pub type UnvalidatedSchema = Schema<UnvalidatedSchemaState>;

pub type UnvalidatedSchemaObject =
    SchemaObject<<UnvalidatedSchemaState as SchemaValidationState>::EncounteredField>;

/// On unvalidated schema objects, the encountered types are either a type annotation
/// for server fields with an unvalidated inner type, or a ScalarFieldName (the name of the
/// resolver.)
pub type UnvalidatedObjectFieldInfo =
    FieldDefinitionLocation<TypeAnnotation<UnvalidatedTypeName>, ClientFieldId>;

pub(crate) type UnvalidatedSchemaData =
    SchemaData<<UnvalidatedSchemaState as SchemaValidationState>::EncounteredField>;

pub(crate) type UnvalidatedSchemaField = SchemaServerField<TypeAnnotation<UnvalidatedTypeName>>;

pub(crate) type UnvalidatedClientField = ClientField<
    <UnvalidatedSchemaState as SchemaValidationState>::ClientFieldSelectionScalarFieldAssociatedData,
    <UnvalidatedSchemaState as SchemaValidationState>::ClientFieldSelectionLinkedFieldAssociatedData,
    <UnvalidatedSchemaState as SchemaValidationState>::ClientFieldVariableDefinitionAssociatedData,
>;

pub type UnvalidatedLinkedFieldSelection = LinkedFieldSelection<
    <UnvalidatedSchemaState as SchemaValidationState>::ClientFieldSelectionScalarFieldAssociatedData,
    <UnvalidatedSchemaState as SchemaValidationState>::ClientFieldSelectionLinkedFieldAssociatedData,
>;

pub(crate) type UnvalidatedSchemaServerField = SchemaServerField<TypeAnnotation<SelectableFieldId>>;

impl UnvalidatedSchema {
    pub fn new() -> Self {
        // TODO add __typename
        let fields = vec![];
        let resolvers = vec![];
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

        Self {
            server_fields: fields,
            client_fields: resolvers,
            entrypoints: Default::default(),
            schema_data: SchemaData {
                objects,
                scalars,
                defined_types,
            },

            id_type_id,
            string_type_id,
            int_type_id,
            float_type_id,
            boolean_type_id,

            query_type_id: None,
        }
    }
}

fn add_schema_defined_scalar_type(
    scalars: &mut Vec<SchemaScalar>,
    defined_types: &mut HashMap<UnvalidatedTypeName, SelectableFieldId>,
    field_name: &'static str,
    javascript_name: JavascriptName,
) -> ScalarId {
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
        SelectableFieldId::Scalar(scalar_id.into()),
    );
    scalar_id
}
