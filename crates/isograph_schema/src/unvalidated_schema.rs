use std::collections::HashMap;

use common_lang_types::{
    JavascriptName, Location, TextSource, UnvalidatedTypeName, WithLocation, WithSpan,
};
use graphql_lang_types::TypeAnnotation;
use intern::string_key::Intern;
use isograph_lang_types::{DefinedTypeId, ResolverFetch, ResolverFieldId, ScalarId};

use crate::{
    DefinedField, Schema, SchemaData, SchemaObject, SchemaResolver, SchemaScalar,
    SchemaServerField, SchemaValidationState,
};
use lazy_static::lazy_static;

lazy_static! {
    pub static ref STRING_JAVASCRIPT_TYPE: JavascriptName = "string".intern().into();
}

#[derive(Debug)]
pub struct UnvalidatedSchemaState {}

impl SchemaValidationState for UnvalidatedSchemaState {
    type FieldAssociatedType = UnvalidatedTypeName;
    type ScalarField = ();
    type LinkedField = ();
    type VariableType = UnvalidatedTypeName;
    type EncounteredField = UnvalidatedObjectFieldInfo;
    type FetchableResolver = (TextSource, WithSpan<ResolverFetch>);
}

pub type UnvalidatedSchema = Schema<UnvalidatedSchemaState>;

pub type UnvalidatedSchemaObject = SchemaObject<UnvalidatedSchemaState>;

/// On unvalidated schema objects, the encountered types are either a type annotation
/// for server fields with an unvalidated inner type, or a ScalarFieldName (the name of the
/// resolver.)
pub type UnvalidatedObjectFieldInfo =
    DefinedField<TypeAnnotation<UnvalidatedTypeName>, ResolverFieldId>;

pub(crate) type UnvalidatedSchemaData = SchemaData<UnvalidatedSchemaState>;

pub(crate) type UnvalidatedSchemaField = SchemaServerField<TypeAnnotation<UnvalidatedTypeName>>;

pub(crate) type UnvalidatedSchemaResolver = SchemaResolver<UnvalidatedSchemaState>;

pub(crate) type UnvalidatedSchemaServerField = SchemaServerField<TypeAnnotation<DefinedTypeId>>;

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
            fields,
            resolvers,
            fetchable_resolvers: Default::default(),
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
    defined_types: &mut HashMap<UnvalidatedTypeName, DefinedTypeId>,
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
        DefinedTypeId::Scalar(scalar_id.into()),
    );
    scalar_id
}
