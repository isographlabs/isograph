use common_lang_types::{
    ConstExportName, IsographDirectiveName, IsographObjectTypeName, LinkedFieldName, Location,
    ObjectTypeAndFieldName, RelativePathToSourceFile, ScalarFieldName, SelectableFieldName,
    TextSource, UnvalidatedTypeName, WithLocation, WithSpan,
};
use intern::string_key::Intern;
use isograph_lang_types::{
    ArgumentKeyAndValue, ClientFieldDeclaration, ClientFieldDeclarationWithValidatedDirectives,
    ClientPointerDeclarationWithValidatedDirectives, ClientPointerId, DeserializationError,
    NonConstantValue, SelectableServerFieldId, ServerObjectId, TypeAnnotation,
};
use lazy_static::lazy_static;

use thiserror::Error;

use crate::{
    refetch_strategy::{generate_refetch_field_strategy, id_selection, RefetchStrategy},
    ClientField, ClientPointer, ClientType, FieldMapItem, FieldType, OutputFormat,
    RequiresRefinement, UnvalidatedSchema, UnvalidatedVariableDefinition, NODE_FIELD_NAME,
};

impl<TOutputFormat: OutputFormat> UnvalidatedSchema<TOutputFormat> {
    pub fn process_client_field_declaration(
        &mut self,
        client_field_declaration: WithSpan<ClientFieldDeclarationWithValidatedDirectives>,
        text_source: TextSource,
    ) -> Result<(), WithLocation<ProcessClientFieldDeclarationError>> {
        let parent_type_id = self
            .server_field_data
            .defined_types
            .get(&client_field_declaration.item.parent_type.item)
            .ok_or(WithLocation::new(
                ProcessClientFieldDeclarationError::ParentTypeNotDefined {
                    parent_type_name: client_field_declaration.item.parent_type.item,
                },
                Location::new(text_source, client_field_declaration.item.parent_type.span),
            ))?;

        match parent_type_id {
            SelectableServerFieldId::Object(object_id) => {
                self.add_client_field_to_object(*object_id, client_field_declaration)
                    .map_err(|e| WithLocation::new(e.item, Location::new(text_source, e.span)))?;
            }
            SelectableServerFieldId::Scalar(scalar_id) => {
                let scalar_name = self.server_field_data.scalar(*scalar_id).name;
                return Err(WithLocation::new(
                    ProcessClientFieldDeclarationError::InvalidParentType {
                        literal_type: "field".to_string(),
                        parent_type_name: scalar_name.item.into(),
                    },
                    Location::new(text_source, client_field_declaration.item.parent_type.span),
                ));
            }
        }

        Ok(())
    }

    pub fn process_client_pointer_declaration(
        &mut self,
        client_pointer_declaration: WithSpan<ClientPointerDeclarationWithValidatedDirectives>,
        text_source: TextSource,
    ) -> Result<(), WithLocation<ProcessClientFieldDeclarationError>> {
        let parent_type_id = self
            .server_field_data
            .defined_types
            .get(&client_pointer_declaration.item.parent_type.item)
            .ok_or(WithLocation::new(
                ProcessClientFieldDeclarationError::ParentTypeNotDefined {
                    parent_type_name: client_pointer_declaration.item.parent_type.item,
                },
                Location::new(
                    text_source,
                    client_pointer_declaration.item.parent_type.span,
                ),
            ))?;

        let target_type_id = self
            .server_field_data
            .defined_types
            .get(client_pointer_declaration.item.target_type.inner())
            .ok_or(WithLocation::new(
                ProcessClientFieldDeclarationError::ParentTypeNotDefined {
                    parent_type_name: *client_pointer_declaration.item.target_type.inner(),
                },
                Location::new(
                    text_source,
                    *client_pointer_declaration.item.target_type.span(),
                ),
            ))?;

        match parent_type_id {
            SelectableServerFieldId::Object(object_id) => match target_type_id {
                SelectableServerFieldId::Object(to_object_id) => {
                    self.add_client_pointer_to_object(
                        *object_id,
                        TypeAnnotation::from_graphql_type_annotation(
                            client_pointer_declaration
                                .item
                                .target_type
                                .clone()
                                .map(|_| *to_object_id),
                        ),
                        client_pointer_declaration,
                    )
                    .map_err(|e| WithLocation::new(e.item, Location::new(text_source, e.span)))?;
                }
                SelectableServerFieldId::Scalar(scalar_id) => {
                    let scalar_name = self.server_field_data.scalar(*scalar_id).name;
                    return Err(WithLocation::new(
                        ProcessClientFieldDeclarationError::ClientPointerInvalidTargetType {
                            target_type_name: scalar_name.item.into(),
                        },
                        Location::new(
                            text_source,
                            *client_pointer_declaration.item.target_type.span(),
                        ),
                    ));
                }
            },
            SelectableServerFieldId::Scalar(scalar_id) => {
                let scalar_name = self.server_field_data.scalar(*scalar_id).name;
                return Err(WithLocation::new(
                    ProcessClientFieldDeclarationError::InvalidParentType {
                        literal_type: "pointer".to_string(),
                        parent_type_name: scalar_name.item.into(),
                    },
                    Location::new(
                        text_source,
                        client_pointer_declaration.item.parent_type.span,
                    ),
                ));
            }
        }

        Ok(())
    }

    fn add_client_field_to_object(
        &mut self,
        parent_object_id: ServerObjectId,
        client_field_declaration: WithSpan<ClientFieldDeclarationWithValidatedDirectives>,
    ) -> ProcessClientFieldDeclarationResult<()> {
        let query_id = self.query_id();
        let object = &mut self.server_field_data.server_objects[parent_object_id.as_usize()];
        let client_field_field_name_ws = client_field_declaration.item.client_field_name;
        let client_field_name = client_field_field_name_ws.item;
        let client_field_name_span = client_field_field_name_ws.span;

        let next_client_field_id = self.client_types.len().into();

        if object
            .encountered_fields
            .insert(
                client_field_name.into(),
                FieldType::ClientField(ClientType::ClientField(next_client_field_id)),
            )
            .is_some()
        {
            // Did not insert, so this object already has a field with the same name :(
            return Err(WithSpan::new(
                ProcessClientFieldDeclarationError::ParentAlreadyHasField {
                    parent_type_name: object.name,
                    client_field_name: client_field_name.into(),
                },
                client_field_name_span,
            ));
        }

        let name = client_field_declaration.item.client_field_name.item.into();
        let variant = get_client_variant(&client_field_declaration.item);

        self.client_types.push(ClientType::ClientField(ClientField {
            description: client_field_declaration.item.description.map(|x| x.item),
            name,
            id: next_client_field_id,
            reader_selection_set: Some(client_field_declaration.item.selection_set),
            variant,
            variable_definitions: client_field_declaration.item.variable_definitions,
            type_and_field: ObjectTypeAndFieldName {
                type_name: object.name,
                field_name: name,
            },

            parent_object_id,
            refetch_strategy: object.id_field.map(|_| {
                // Assume that if we have an id field, this implements Node
                RefetchStrategy::UseRefetchField(generate_refetch_field_strategy(
                    vec![id_selection()],
                    query_id,
                    format!("refetch__{}", object.name).intern().into(),
                    *NODE_FIELD_NAME,
                    id_top_level_arguments(),
                    None,
                    RequiresRefinement::Yes(object.name),
                    None,
                    None,
                ))
            }),
            output_format: std::marker::PhantomData,
        }));
        Ok(())
    }

    fn add_client_pointer_to_object(
        &mut self,
        parent_object_id: ServerObjectId,
        to_object_id: TypeAnnotation<ServerObjectId>,
        client_pointer_declaration: WithSpan<ClientPointerDeclarationWithValidatedDirectives>,
    ) -> ProcessClientFieldDeclarationResult<()> {
        let query_id = self.query_id();
        let to_object = self.server_field_data.object(to_object_id.inner());
        let parent_object = self.server_field_data.object(parent_object_id);
        let client_pointer_pointer_name_ws = client_pointer_declaration.item.client_pointer_name;
        let client_pointer_name = client_pointer_pointer_name_ws.item;
        let client_pointer_name_span = client_pointer_pointer_name_ws.span;

        let next_client_pointer_id: ClientPointerId = self.client_types.len().into();

        let name = client_pointer_declaration.item.client_pointer_name.item;

        if let Some(directive) = client_pointer_declaration
            .item
            .directives
            .into_iter()
            .next()
        {
            return Err(directive.map(|directive| {
                ProcessClientFieldDeclarationError::DirectiveNotSupportedOnClientPointer {
                    directive_name: directive.name.item,
                }
            }));
        }

        self.client_types
            .push(ClientType::ClientPointer(ClientPointer {
                description: client_pointer_declaration.item.description.map(|x| x.item),
                name,
                id: next_client_pointer_id,
                reader_selection_set: client_pointer_declaration.item.selection_set,

                variable_definitions: client_pointer_declaration.item.variable_definitions,
                type_and_field: ObjectTypeAndFieldName {
                    type_name: parent_object.name,
                    field_name: name.into(),
                },

                parent_object_id,
                refetch_strategy: match to_object.id_field {
                    None => Err(WithSpan::new(
                        ProcessClientFieldDeclarationError::ClientPointerTargetTypeHasNoId {
                            target_type_name: *client_pointer_declaration.item.target_type.inner(),
                        },
                        *client_pointer_declaration.item.target_type.span(),
                    )),
                    Some(_) => {
                        // Assume that if we have an id field, this implements Node
                        Ok(RefetchStrategy::UseRefetchField(
                            generate_refetch_field_strategy(
                                vec![id_selection()],
                                query_id,
                                format!("refetch__{}", to_object.name).intern().into(),
                                *NODE_FIELD_NAME,
                                id_top_level_arguments(),
                                None,
                                RequiresRefinement::Yes(to_object.name),
                                None,
                                None,
                            ),
                        ))
                    }
                }?,
                to: to_object_id,
                output_format: std::marker::PhantomData,
            }));

        let parent_object = self.server_field_data.object_mut(parent_object_id);
        if parent_object
            .encountered_fields
            .insert(
                client_pointer_name.into(),
                FieldType::ClientField(ClientType::ClientPointer(next_client_pointer_id)),
            )
            .is_some()
        {
            // Did not insert, so this object already has a field with the same name :(
            return Err(WithSpan::new(
                ProcessClientFieldDeclarationError::ParentAlreadyHasField {
                    parent_type_name: parent_object.name,
                    client_field_name: client_pointer_name.into(),
                },
                client_pointer_name_span,
            ));
        }

        Ok(())
    }
}

type ProcessClientFieldDeclarationResult<T> =
    Result<T, WithSpan<ProcessClientFieldDeclarationError>>;

#[derive(Error, Eq, PartialEq, Debug)]
pub enum ProcessClientFieldDeclarationError {
    #[error("`{parent_type_name}` is not a type that has been defined.")]
    ParentTypeNotDefined {
        parent_type_name: UnvalidatedTypeName,
    },

    #[error("Directive {directive_name} is not supported on client pointers.")]
    DirectiveNotSupportedOnClientPointer {
        directive_name: IsographDirectiveName,
    },

    #[error("Invalid parent type. `{parent_type_name}` is a scalar. You are attempting to define a {literal_type} on it. \
        In order to do so, the parent object must be an object, interface or union.")]
    InvalidParentType {
        literal_type: String,
        parent_type_name: UnvalidatedTypeName,
    },

    #[error("Invalid client pointer target type. `{target_type_name}` is a scalar. You are attempting to define a pointer to it. \
        In order to do so, the type must be an object, interface or union.")]
    ClientPointerInvalidTargetType {
        target_type_name: UnvalidatedTypeName,
    },

    #[error("Invalid client pointer target type. `{target_type_name}` has no id field. You are attempting to define a pointer to it. \
        In order to do so, the target must be an object implementing Node interface.")]
    ClientPointerTargetTypeHasNoId {
        target_type_name: UnvalidatedTypeName,
    },

    #[error(
        "The Isograph object type \"{parent_type_name}\" already has a field named \"{client_field_name}\"."
    )]
    ParentAlreadyHasField {
        parent_type_name: IsographObjectTypeName,
        client_field_name: SelectableFieldName,
    },

    #[error("Unable to serialize directive named \"@{directive_name}\". Message: {message}")]
    UnableToDeserialize {
        directive_name: IsographDirectiveName,
        message: DeserializationError,
    },

    #[error("The directive \"@loadable\" and \"@updatable\" are mutually exclusive.")]
    LoadableAndUpdatableAreMutuallyExclusive,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PrimaryFieldInfo {
    pub primary_field_name: LinkedFieldName,
    /// Some if the object is concrete; None otherwise.
    pub primary_field_concrete_type: Option<IsographObjectTypeName>,
    /// If this is abstract, we add a fragment spread
    pub primary_field_return_type_object_id: ServerObjectId,
    pub primary_field_field_map: Vec<FieldMapItem>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ImperativelyLoadedFieldVariant {
    pub client_field_scalar_selection_name: ScalarFieldName,
    /// What field should we select when generating the refetch query?
    pub top_level_schema_field_name: LinkedFieldName,
    /// The arguments we must pass to the top level schema field, e.g. id: ID!
    /// for node(id: $id)
    pub top_level_schema_field_arguments: Vec<UnvalidatedVariableDefinition>,

    /// Some if the object is concrete; None otherwise.
    pub top_level_schema_field_concrete_type: Option<IsographObjectTypeName>,

    /// If we need to select a sub-field, this is Some(...). We should model
    /// this differently, this is very awkward!
    pub primary_field_info: Option<PrimaryFieldInfo>,

    pub root_object_id: ServerObjectId,
}

// TODO Component is a GraphQL-ism
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum UserWrittenComponentVariant {
    Eager,
    Component,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UserWrittenClientFieldInfo {
    // TODO use a shared struct
    pub const_export_name: ConstExportName,
    pub file_path: RelativePathToSourceFile,
    pub user_written_component_variant: UserWrittenComponentVariant,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ClientFieldVariant {
    UserWritten(UserWrittenClientFieldInfo),
    ImperativelyLoadedField(ImperativelyLoadedFieldVariant),
    Link,
}

lazy_static! {
    static ref COMPONENT: IsographDirectiveName = "component".intern().into();
}

fn get_client_variant<TScalarField, TLinkedField>(
    client_field_declaration: &ClientFieldDeclaration<TScalarField, TLinkedField>,
) -> ClientFieldVariant {
    for directive in client_field_declaration.directives.iter() {
        if directive.item.name.item == *COMPONENT {
            return ClientFieldVariant::UserWritten(UserWrittenClientFieldInfo {
                const_export_name: client_field_declaration.const_export_name,
                file_path: client_field_declaration.definition_path,
                user_written_component_variant: UserWrittenComponentVariant::Component,
            });
        }
    }
    ClientFieldVariant::UserWritten(UserWrittenClientFieldInfo {
        const_export_name: client_field_declaration.const_export_name,
        file_path: client_field_declaration.definition_path,
        user_written_component_variant: UserWrittenComponentVariant::Eager,
    })
}

pub fn id_top_level_arguments() -> Vec<ArgumentKeyAndValue> {
    vec![ArgumentKeyAndValue {
        key: "id".intern().into(),
        value: NonConstantValue::Variable("id".intern().into()),
    }]
}
