use common_lang_types::{ServerSelectableName, Span, UnvalidatedTypeName, WithLocation, WithSpan};
use graphql_lang_types::{GraphQLNamedTypeAnnotation, GraphQLTypeAnnotation};
use intern::string_key::Intern;
use isograph_lang_types::{
    DefinitionLocation, EmptyDirectiveSet, ScalarFieldSelection, ScalarSelectionDirectiveSet,
    SelectionType, ServerFieldSelection, TypeAnnotation,
};

use crate::{
    as_client_type, as_server_field, OutputFormat, SchemaServerField,
    SchemaServerLinkedFieldFieldVariant, ServerFieldTypeAssociatedDataInlineFragment,
    UnvalidatedSchema, ValidatedScalarSelectionAssociatedData, LINK_FIELD_NAME,
};
use common_lang_types::Location;

use super::create_additional_fields_error::{
    CreateAdditionalFieldsError, ProcessTypeDefinitionResult, ValidatedTypeRefinementMap,
};
impl<TOutputFormat: OutputFormat> UnvalidatedSchema<TOutputFormat> {
    /// For each supertype (e.g. Node), add the pointers for each subtype (e.g. User implements Node)
    /// to supertype (e.g. creating Node.asUser).
    pub fn add_pointers_to_supertypes(
        &mut self,
        subtype_to_supertype_map: &ValidatedTypeRefinementMap,
    ) -> ProcessTypeDefinitionResult<()> {
        for (subtype_id, supertype_ids) in subtype_to_supertype_map {
            let subtype = self.server_field_data.object(*subtype_id);

            if let Some(concrete_type) = subtype.concrete_type {
                let field_name: ServerSelectableName =
                    format!("as{}", subtype.name).intern().into();

                let next_server_field_id = self.server_fields.len().into();

                let graphql_type_annotation: GraphQLTypeAnnotation<UnvalidatedTypeName> =
                    GraphQLTypeAnnotation::Named(GraphQLNamedTypeAnnotation(WithSpan {
                        item: subtype.name.into(),
                        span: Span::todo_generated(),
                    }));

                let typename_selection = WithSpan::new(
                    ServerFieldSelection::ScalarField(ScalarFieldSelection {
                        arguments: vec![],
                        associated_data: ValidatedScalarSelectionAssociatedData {
                            location: DefinitionLocation::Server(
                                *as_server_field(
                                    subtype
                                        .encountered_fields
                                        .get(&"__typename".intern().into())
                                        .expect("Expected __typename to exist"),
                                )
                                .expect("Expected __typename to be server field"),
                            ),
                            selection_variant: ScalarSelectionDirectiveSet::None(
                                EmptyDirectiveSet {},
                            ),
                        },
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
                        associated_data: ValidatedScalarSelectionAssociatedData {
                            location: DefinitionLocation::Client(
                                match *as_client_type(
                                    subtype
                                        .encountered_fields
                                        .get(&(*LINK_FIELD_NAME).into())
                                        .expect("Expected link to exist"),
                                )
                                .expect("Expected link to be client field")
                                {
                                    SelectionType::Scalar(client_field_id) => client_field_id,
                                    SelectionType::Object(_) => {
                                        panic!("Expected link to be client field")
                                    }
                                },
                            ),
                            selection_variant: ScalarSelectionDirectiveSet::None(
                                EmptyDirectiveSet {},
                            ),
                        },
                        name: WithLocation::new((*LINK_FIELD_NAME).into(), Location::generated()),
                        reader_alias: None,
                    }),
                    Span::todo_generated(),
                );

                let reader_selection_set = vec![typename_selection, link_selection];

                let target_object = TypeAnnotation::from_graphql_type_annotation(
                    graphql_type_annotation.map(|_| SelectionType::Object(*subtype_id)),
                );

                // TODO ... is this a server field? Yes, because it's an inline fragment?
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
                    linked_field_variant: SchemaServerLinkedFieldFieldVariant::InlineFragment(
                        ServerFieldTypeAssociatedDataInlineFragment {
                            server_field_id: next_server_field_id,
                            concrete_type,
                            reader_selection_set,
                        },
                    ),
                    target_server_entity: target_object,
                    phantom_data: std::marker::PhantomData,
                };

                self.server_fields.push(server_field);

                for supertype_id in supertype_ids {
                    let supertype = self.server_field_data.object_mut(*supertype_id);

                    if supertype
                        .encountered_fields
                        .insert(
                            field_name.into(),
                            DefinitionLocation::Server(next_server_field_id),
                        )
                        .is_some()
                    {
                        return Err(WithLocation::new(
                            CreateAdditionalFieldsError::FieldExistsOnType {
                                field_name: field_name.into(),
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
