use common_lang_types::{SelectableFieldName, Span, UnvalidatedTypeName, WithLocation, WithSpan};
use graphql_lang_types::{GraphQLNamedTypeAnnotation, GraphQLTypeAnnotation};
use intern::string_key::Intern;
use isograph_lang_types::{ScalarFieldSelection, ServerFieldSelection};

use crate::{
    ClientType, FieldType, ProcessTypeDefinitionError, ProcessTypeDefinitionResult, SchemaObject,
    SchemaServerField, SchemaServerFieldVariant, ServerFieldTypeAssociatedData,
    ServerFieldTypeAssociatedDataInlineFragment, UnvalidatedSchema,
    ValidatedIsographSelectionVariant, ValidatedScalarFieldAssociatedData,
    ValidatedTypeRefinementMap, LINK_FIELD_NAME,
};
use common_lang_types::Location;
impl UnvalidatedSchema {
    /// For each supertype (e.g. Node), add the pointers for each subtype (e.g. User implements Node)
    /// to supertype (e.g. creating Node.asUser).
    pub fn add_pointers_to_supertypes(
        &mut self,
        subtype_to_supertype_map: &ValidatedTypeRefinementMap,
    ) -> ProcessTypeDefinitionResult<()> {
        for (subtype_id, supertype_ids) in subtype_to_supertype_map {
            let subtype: &SchemaObject = self.server_field_data.object(*subtype_id);

            if let Some(concrete_type) = subtype.concrete_type {
                let field_name: SelectableFieldName = format!("as{}", subtype.name).intern().into();

                let next_server_field_id = self.server_fields.len().into();

                let associated_data: GraphQLTypeAnnotation<UnvalidatedTypeName> =
                    GraphQLTypeAnnotation::Named(GraphQLNamedTypeAnnotation(WithSpan {
                        item: subtype.name.into(),
                        span: Span::todo_generated(),
                    }));

                let typename_selection = WithSpan::new(
                    ServerFieldSelection::ScalarField(ScalarFieldSelection {
                        arguments: vec![],
                        associated_data: ValidatedScalarFieldAssociatedData {
                            location: FieldType::ServerField(
                                *subtype
                                    .encountered_fields
                                    .get(&"__typename".intern().into())
                                    .expect("Expected __typename to exist")
                                    .as_server_field()
                                    .expect("Expected __typename to be server field"),
                            ),
                            selection_variant: ValidatedIsographSelectionVariant::Regular,
                        },
                        directives: vec![],
                        name: WithLocation::new(
                            "__typename".intern().into(),
                            Location::generated(),
                        ),
                        reader_alias: None,
                    }),
                    Span::todo_generated(),
                );

                let link_selection = WithSpan::new(
                    ServerFieldSelection::ScalarField(ScalarFieldSelection {
                        arguments: vec![],
                        associated_data: ValidatedScalarFieldAssociatedData {
                            location: FieldType::ClientField(
                                match *subtype
                                    .encountered_fields
                                    .get(&(*LINK_FIELD_NAME).into())
                                    .expect("Expected link to exist")
                                    .as_client_type()
                                    .expect("Expected link to be client field")
                                {
                                    ClientType::ClientField(client_field_id) => client_field_id,
                                    ClientType::ClientPointer(_) => {
                                        panic!("Expected link to be client field")
                                    }
                                },
                            ),
                            selection_variant: ValidatedIsographSelectionVariant::Regular,
                        },
                        directives: vec![],
                        name: WithLocation::new(*LINK_FIELD_NAME, Location::generated()),
                        reader_alias: None,
                    }),
                    Span::todo_generated(),
                );

                let reader_selection_set = vec![typename_selection, link_selection];

                let server_field = SchemaServerField {
                    description: Some(
                        format!("A client pointer for the {} type.", subtype.name)
                            .intern()
                            .into(),
                    ),
                    id: next_server_field_id,
                    name: WithLocation::new(field_name, Location::generated()),
                    parent_type_id: subtype.id,
                    arguments: vec![],
                    associated_data: ServerFieldTypeAssociatedData {
                        type_name: associated_data,
                        variant: SchemaServerFieldVariant::InlineFragment(
                            ServerFieldTypeAssociatedDataInlineFragment {
                                server_field_id: next_server_field_id,
                                concrete_type,
                                reader_selection_set,
                            },
                        ),
                    },
                    is_discriminator: false,
                };

                self.server_fields.push(server_field);

                for supertype_id in supertype_ids {
                    let supertype = self.server_field_data.object_mut(*supertype_id);

                    if supertype
                        .encountered_fields
                        .insert(field_name, FieldType::ServerField(next_server_field_id))
                        .is_some()
                    {
                        return Err(WithLocation::new(
                            ProcessTypeDefinitionError::FieldExistsOnType {
                                field_name,
                                parent_type: supertype.name,
                            },
                            Location::generated(),
                        ));
                    }
                }
            }
        }
        Ok(())
    }
}
