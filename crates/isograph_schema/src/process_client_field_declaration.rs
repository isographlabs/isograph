use std::collections::HashMap;

use common_lang_types::{
    ClientObjectSelectableName, ClientScalarSelectableName, ConstExportName, IsographDirectiveName,
    Location, ObjectTypeAndFieldName, RelativePathToSourceFile, SelectableName,
    ServerObjectEntityName, TextSource, UnvalidatedTypeName, VariableName, WithLocation, WithSpan,
};
use intern::string_key::Intern;
use isograph_lang_types::{
    ArgumentKeyAndValue, ClientFieldDeclaration, ClientFieldDirectiveSet, ClientPointerDeclaration,
    DefinitionLocation, DeserializationError, NonConstantValue, SelectionType,
    ServerObjectEntityNameWrapper, TypeAnnotation, UnvalidatedSelection, VariableDefinition,
};

use thiserror::Error;

use crate::{
    refetch_strategy::{generate_refetch_field_strategy, id_selection, RefetchStrategy},
    ClientObjectSelectable, ClientScalarSelectable, FieldMapItem, NetworkProtocol, Schema,
    ServerEntityName, ValidatedVariableDefinition, WrappedSelectionMapSelection, NODE_FIELD_NAME,
};

pub type UnprocessedSelection = WithSpan<UnvalidatedSelection>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnprocessedClientFieldItem {
    pub parent_object_entity_name: ServerObjectEntityName,
    pub client_field_name: ClientScalarSelectableName,
    pub reader_selection_set: Vec<UnprocessedSelection>,
    pub refetch_strategy: Option<RefetchStrategy<(), ()>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnprocessedClientPointerItem {
    pub parent_object_entity_name: ServerObjectEntityName,
    pub client_object_selectable_name: ClientObjectSelectableName,
    pub reader_selection_set: Vec<UnprocessedSelection>,
    pub refetch_selection_set: Vec<UnprocessedSelection>,
}
pub type UnprocessedItem = SelectionType<UnprocessedClientFieldItem, UnprocessedClientPointerItem>;

impl<TNetworkProtocol: NetworkProtocol> Schema<TNetworkProtocol> {
    pub fn process_client_field_declaration(
        &mut self,
        client_field_declaration: WithSpan<ClientFieldDeclaration>,
        text_source: TextSource,
    ) -> Result<UnprocessedClientFieldItem, WithLocation<ProcessClientFieldDeclarationError>> {
        let parent_type_id = self
            .server_entity_data
            .defined_entities
            .get(&client_field_declaration.item.parent_type.item.0)
            .ok_or(WithLocation::new(
                ProcessClientFieldDeclarationError::ParentTypeNotDefined {
                    parent_type_name: client_field_declaration.item.parent_type.item,
                },
                Location::new(text_source, client_field_declaration.item.parent_type.span),
            ))?;

        let unprocess_client_field_items = match parent_type_id {
            ServerEntityName::Object(object_entity_name) => self
                .add_client_field_to_object(*object_entity_name, client_field_declaration)
                .map_err(|e| WithLocation::new(e.item, Location::new(text_source, e.span)))?,
            ServerEntityName::Scalar(scalar_entity_name) => {
                let scalar_name = self
                    .server_entity_data
                    .server_scalar_entity(*scalar_entity_name)
                    .expect(
                        "Expected entity to exist. \
                        This is indicative of a bug in Isograph.",
                    )
                    .name;
                return Err(WithLocation::new(
                    ProcessClientFieldDeclarationError::InvalidParentType {
                        literal_type: "field".to_string(),
                        parent_type_name: scalar_name.item.into(),
                    },
                    Location::new(text_source, client_field_declaration.item.parent_type.span),
                ));
            }
        };

        Ok(unprocess_client_field_items)
    }

    pub fn process_client_pointer_declaration(
        &mut self,
        client_pointer_declaration: WithSpan<ClientPointerDeclaration>,
        text_source: TextSource,
    ) -> Result<UnprocessedClientPointerItem, WithLocation<ProcessClientFieldDeclarationError>>
    {
        let parent_type_id = self
            .server_entity_data
            .defined_entities
            .get(&client_pointer_declaration.item.parent_type.item.0)
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
            .server_entity_data
            .defined_entities
            .get(client_pointer_declaration.item.target_type.inner())
            .ok_or(WithLocation::new(
                ProcessClientFieldDeclarationError::ParentTypeNotDefined {
                    parent_type_name: ServerObjectEntityNameWrapper(
                        *client_pointer_declaration.item.target_type.inner(),
                    ),
                },
                Location::new(
                    text_source,
                    client_pointer_declaration.item.target_type.span(),
                ),
            ))?;

        let unprocessed_client_pointer_items = match parent_type_id {
            ServerEntityName::Object(object_entity_name) => match target_type_id {
                ServerEntityName::Object(to_object_entity_name) => self
                    .add_client_pointer_to_object(
                        *object_entity_name,
                        TypeAnnotation::from_graphql_type_annotation(
                            client_pointer_declaration
                                .item
                                .target_type
                                .clone()
                                .map(|_| *to_object_entity_name),
                        ),
                        client_pointer_declaration,
                    )
                    .map_err(|e| WithLocation::new(e.item, Location::new(text_source, e.span)))?,
                ServerEntityName::Scalar(scalar_entity_name) => {
                    let scalar_name = self
                        .server_entity_data
                        .server_scalar_entity(*scalar_entity_name)
                        .expect(
                            "Expected entity to exist. \
                            This is indicative of a bug in Isograph.",
                        )
                        .name;
                    return Err(WithLocation::new(
                        ProcessClientFieldDeclarationError::ClientPointerInvalidTargetType {
                            target_type_name: scalar_name.item.into(),
                        },
                        Location::new(
                            text_source,
                            client_pointer_declaration.item.target_type.span(),
                        ),
                    ));
                }
            },
            ServerEntityName::Scalar(scalar_entity_name) => {
                let scalar_name = self
                    .server_entity_data
                    .server_scalar_entity(*scalar_entity_name)
                    .expect(
                        "Expected entity to exist. \
                        This is indicative of a bug in Isograph.",
                    )
                    .name;
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
        };
        Ok(unprocessed_client_pointer_items)
    }

    fn add_client_field_to_object(
        &mut self,
        parent_object_entity_name: ServerObjectEntityName,
        client_field_declaration: WithSpan<ClientFieldDeclaration>,
    ) -> ProcessClientFieldDeclarationResult<UnprocessedClientFieldItem> {
        let query_id = self.query_id();
        let object = &mut self
            .server_entity_data
            .server_objects
            .get(&parent_object_entity_name)
            .expect("Expected type to exist");
        let client_field_field_name_ws = client_field_declaration.item.client_field_name;
        let client_field_name = client_field_field_name_ws.item;
        let client_field_name_span = client_field_field_name_ws.span;
        let client_scalar_selectable_name = client_field_declaration.item.client_field_name.item;

        if self
            .server_entity_data
            .server_object_entity_extra_info
            .entry(parent_object_entity_name)
            .or_default()
            .selectables
            .insert(
                client_field_name.0.into(),
                DefinitionLocation::Client(SelectionType::Scalar((
                    parent_object_entity_name,
                    client_scalar_selectable_name.0,
                ))),
            )
            .is_some()
        {
            // Did not insert, so this object already has a field with the same name :(
            return Err(WithSpan::new(
                ProcessClientFieldDeclarationError::ParentAlreadyHasField {
                    parent_type_name: object.name,
                    client_field_name: client_field_name.0.into(),
                },
                client_field_name_span,
            ));
        }

        let variant = get_client_variant(&client_field_declaration.item);

        self.client_scalar_selectables.insert(
            (object.name, client_scalar_selectable_name.0),
            ClientScalarSelectable {
                description: client_field_declaration.item.description.map(|x| x.item),
                name: client_scalar_selectable_name.0,
                reader_selection_set: vec![],
                variant,
                variable_definitions: client_field_declaration
                    .item
                    .variable_definitions
                    .into_iter()
                    .map(|variable_definition| {
                        validate_variable_definition(
                            &self.server_entity_data.defined_entities,
                            variable_definition,
                            object.name,
                            client_field_name.0.into(),
                        )
                    })
                    .collect::<Result<_, _>>()?,
                type_and_field: ObjectTypeAndFieldName {
                    type_name: object.name,
                    field_name: client_scalar_selectable_name.0.into(),
                },

                parent_object_entity_name,
                refetch_strategy: None,
                network_protocol: std::marker::PhantomData,
            },
        );

        let selections = client_field_declaration.item.selection_set;
        let id_field = self
            .server_entity_data
            .server_object_entity_extra_info
            .get(&parent_object_entity_name)
            .expect(
                "Expected parent_object_entity_name to exist in \
                server_object_entity_available_selectables",
            )
            .id_field;
        let refetch_strategy = id_field.map(|_| {
            // Assume that if we have an id field, this implements Node
            RefetchStrategy::UseRefetchField(generate_refetch_field_strategy(
                vec![id_selection()],
                query_id,
                vec![
                    WrappedSelectionMapSelection::InlineFragment(object.name),
                    WrappedSelectionMapSelection::LinkedField {
                        server_object_selectable_name: *NODE_FIELD_NAME,
                        arguments: id_top_level_arguments(),
                        concrete_type: None,
                    },
                ],
            ))
        });

        Ok(UnprocessedClientFieldItem {
            parent_object_entity_name,
            client_field_name: *client_scalar_selectable_name,
            reader_selection_set: selections,
            refetch_strategy,
        })
    }

    fn add_client_pointer_to_object(
        &mut self,
        parent_object_name: ServerObjectEntityName,
        to_object_name: TypeAnnotation<ServerObjectEntityName>,
        client_pointer_declaration: WithSpan<ClientPointerDeclaration>,
    ) -> ProcessClientFieldDeclarationResult<UnprocessedClientPointerItem> {
        let query_id = self.query_id();
        let to_object = self
            .server_entity_data
            .server_object_entity(*to_object_name.inner())
            .expect(
                "Expected entity to exist. \
                This is indicative of a bug in Isograph.",
            );
        let parent_object = self
            .server_entity_data
            .server_object_entity(parent_object_name)
            .expect(
                "Expected entity to exist. \
                This is indicative of a bug in Isograph.",
            );
        let client_pointer_pointer_name_ws = client_pointer_declaration.item.client_pointer_name;
        let client_pointer_name = client_pointer_pointer_name_ws.item;
        let client_pointer_name_span = client_pointer_pointer_name_ws.span;

        let client_object_selectable_name =
            client_pointer_declaration.item.client_pointer_name.item;

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

        let unprocessed_fields = client_pointer_declaration.item.selection_set;

        let id_field = self
            .server_entity_data
            .server_object_entity_extra_info
            .get(&parent_object_name)
            .expect(
                "Expected parent_object_entity_name \
                to exist in server_object_entity_available_selectables",
            )
            .id_field;
        let refetch_strategy = match id_field {
            None => Err(WithSpan::new(
                ProcessClientFieldDeclarationError::ClientPointerTargetTypeHasNoId {
                    target_type_name: *client_pointer_declaration.item.target_type.inner(),
                },
                client_pointer_declaration.item.target_type.span(),
            )),
            Some(_) => {
                // Assume that if we have an id field, this implements Node
                Ok(RefetchStrategy::UseRefetchField(
                    generate_refetch_field_strategy(
                        vec![],
                        query_id,
                        vec![
                            WrappedSelectionMapSelection::InlineFragment(to_object.name),
                            WrappedSelectionMapSelection::LinkedField {
                                server_object_selectable_name: *NODE_FIELD_NAME,
                                arguments: id_top_level_arguments(),
                                concrete_type: None,
                            },
                        ],
                    ),
                ))
            }
        }?;

        self.client_object_selectables.insert(
            (parent_object_name, *client_object_selectable_name),
            ClientObjectSelectable {
                description: client_pointer_declaration.item.description.map(|x| x.item),
                name: *client_object_selectable_name,
                reader_selection_set: vec![],

                variable_definitions: client_pointer_declaration
                    .item
                    .variable_definitions
                    .into_iter()
                    .map(|variable_definition| {
                        validate_variable_definition(
                            &self.server_entity_data.defined_entities,
                            variable_definition,
                            parent_object.name,
                            client_pointer_name.0.into(),
                        )
                    })
                    .collect::<Result<_, _>>()?,
                type_and_field: ObjectTypeAndFieldName {
                    type_name: parent_object.name,
                    field_name: client_object_selectable_name.0.into(),
                },

                parent_object_name,
                refetch_strategy,
                target_object_entity_name: to_object_name,
                network_protocol: std::marker::PhantomData,

                info: UserWrittenClientPointerInfo {
                    const_export_name: client_pointer_declaration.item.const_export_name,
                    file_path: client_pointer_declaration.item.definition_path,
                },
            },
        );

        if self
            .server_entity_data
            .server_object_entity_extra_info
            .entry(parent_object_name)
            .or_default()
            .selectables
            .insert(
                client_pointer_name.0.into(),
                DefinitionLocation::Client(SelectionType::Object((
                    parent_object_name,
                    *client_object_selectable_name,
                ))),
            )
            .is_some()
        {
            let parent_object = self
                .server_entity_data
                .server_object_entity(parent_object_name)
                .expect(
                    "Expected entity to exist. \
                    This is indicative of a bug in Isograph.",
                );
            // Did not insert, so this object already has a field with the same name :(
            return Err(WithSpan::new(
                ProcessClientFieldDeclarationError::ParentAlreadyHasField {
                    parent_type_name: parent_object.name,
                    client_field_name: client_pointer_name.0.into(),
                },
                client_pointer_name_span,
            ));
        }

        Ok(UnprocessedClientPointerItem {
            client_object_selectable_name: *client_pointer_name,
            parent_object_entity_name: parent_object_name,
            reader_selection_set: unprocessed_fields,
            refetch_selection_set: vec![id_selection()],
        })
    }
}

type ProcessClientFieldDeclarationResult<T> =
    Result<T, WithSpan<ProcessClientFieldDeclarationError>>;

#[derive(Error, Eq, PartialEq, Debug, Clone)]
pub enum ProcessClientFieldDeclarationError {
    #[error("`{parent_type_name}` is not a type that has been defined.")]
    ParentTypeNotDefined {
        parent_type_name: ServerObjectEntityNameWrapper,
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
        parent_type_name: ServerObjectEntityName,
        client_field_name: SelectableName,
    },

    #[error("Error when deserializing directives. Message: {message}")]
    UnableToDeserializeDirectives { message: DeserializationError },

    #[error(
        "The argument `{argument_name}` on field `{parent_type_name}.{field_name}` has inner type `{argument_type}`, which does not exist."
    )]
    FieldArgumentTypeDoesNotExist {
        argument_name: VariableName,
        parent_type_name: ServerObjectEntityName,
        field_name: SelectableName,
        argument_type: UnvalidatedTypeName,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ImperativelyLoadedFieldVariant {
    pub client_field_scalar_selection_name: ClientScalarSelectableName,

    // Mutation or Query or whatnot. Awkward! A GraphQL-ism!
    pub root_object_entity_name: ServerObjectEntityName,
    pub subfields_or_inline_fragments: Vec<WrappedSelectionMapSelection>,
    pub field_map: Vec<FieldMapItem>,
    /// The arguments we must pass to the top level schema field, e.g. id: ID!
    /// for node(id: $id). These are already encoded in the subfields_or_inline_fragments,
    /// but we nonetheless need to put them into the query definition, and we need
    /// the variable's type, not just the variable.
    pub top_level_schema_field_arguments: Vec<ValidatedVariableDefinition>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UserWrittenClientTypeInfo {
    // TODO use a shared struct
    pub const_export_name: ConstExportName,
    pub file_path: RelativePathToSourceFile,
    pub client_field_directive_set: ClientFieldDirectiveSet,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
// TODO refactor this https://github.com/isographlabs/isograph/pull/435#discussion_r1970489356
pub struct UserWrittenClientPointerInfo {
    pub const_export_name: ConstExportName,
    pub file_path: RelativePathToSourceFile,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ClientFieldVariant {
    UserWritten(UserWrittenClientTypeInfo),
    ImperativelyLoadedField(ImperativelyLoadedFieldVariant),
    Link,
}

fn get_client_variant(client_field_declaration: &ClientFieldDeclaration) -> ClientFieldVariant {
    ClientFieldVariant::UserWritten(UserWrittenClientTypeInfo {
        const_export_name: client_field_declaration.const_export_name,
        file_path: client_field_declaration.definition_path,
        client_field_directive_set: client_field_declaration.client_field_directive_set,
    })
}

pub fn id_top_level_arguments() -> Vec<ArgumentKeyAndValue> {
    vec![ArgumentKeyAndValue {
        key: "id".intern().into(),
        value: NonConstantValue::Variable("id".intern().into()),
    }]
}

pub fn validate_variable_definition(
    defined_types: &HashMap<UnvalidatedTypeName, ServerEntityName>,
    variable_definition: WithSpan<VariableDefinition<UnvalidatedTypeName>>,
    parent_type_name: ServerObjectEntityName,
    field_name: SelectableName,
) -> ProcessClientFieldDeclarationResult<WithSpan<VariableDefinition<ServerEntityName>>> {
    let type_ = variable_definition
        .item
        .type_
        .clone()
        .and_then(|input_type_name| {
            defined_types
                .get(variable_definition.item.type_.inner())
                .ok_or_else(|| {
                    WithSpan::new(
                        ProcessClientFieldDeclarationError::FieldArgumentTypeDoesNotExist {
                            argument_type: input_type_name,
                            argument_name: variable_definition.item.name.item,
                            parent_type_name,
                            field_name,
                        },
                        variable_definition.span,
                    )
                })
                .copied()
        })?;

    Ok(WithSpan::new(
        VariableDefinition {
            name: variable_definition.item.name.map(VariableName::from),
            type_,
            default_value: variable_definition.item.default_value,
        },
        variable_definition.span,
    ))
}
