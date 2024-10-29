use common_lang_types::{SelectableFieldName, Span, UnvalidatedTypeName, WithLocation};
use graphql_lang_types::GraphQLTypeAnnotation;
use intern::string_key::Intern;

use crate::{
    ProcessTypeDefinitionError, ProcessTypeDefinitionResult, SchemaServerField, TypeRefinementMap,
    UnvalidatedSchema,
};
use common_lang_types::Location;
impl UnvalidatedSchema {
    /// For each supertype (e.g. Node), add the pointers for each subtype (e.g. User implements Node)
    /// to supertype (e.g. creating Node.asUser).
    pub fn add_pointers_to_supertypes(
        &mut self,
        subtype_to_supertype_map: &TypeRefinementMap,
    ) -> ProcessTypeDefinitionResult<()> {
        for (subtype_id, supertype_ids) in subtype_to_supertype_map {
            let subtype: &crate::SchemaObject = self.server_field_data.object(*subtype_id);

            let field_name: SelectableFieldName = format!("as{}", subtype.name).intern().into();

            let next_server_field_id = self.server_fields.len().into();

            let associated_data: GraphQLTypeAnnotation<UnvalidatedTypeName> =
                GraphQLTypeAnnotation::Named(graphql_lang_types::GraphQLNamedTypeAnnotation(
                    common_lang_types::WithSpan {
                        item: subtype.name.into(),
                        span: Span::todo_generated(),
                    },
                ));

            let server_field = SchemaServerField {
                description: Some(
                    format!("A client poiter for the {} type.", subtype.name)
                        .intern()
                        .into(),
                ),
                id: next_server_field_id,
                name: WithLocation::new(field_name, Location::generated()),
                parent_type_id: subtype.id,
                arguments: vec![],
                associated_data,
                is_discriminator: false,
            };

            self.server_fields.push(server_field);

            for supertype_id in supertype_ids {
                let supertype = self.server_field_data.object_mut(*supertype_id);

                if supertype
                    .encountered_fields
                    .insert(
                        field_name.into(),
                        crate::FieldType::ServerField(next_server_field_id),
                    )
                    .is_some()
                {
                    return Err(WithLocation::new(
                        ProcessTypeDefinitionError::FieldExistsOnSubtype {
                            field_name,
                            parent_type: supertype.name,
                        },
                        Location::generated(),
                    ));
                }
            }
        }
        Ok(())
    }
}
