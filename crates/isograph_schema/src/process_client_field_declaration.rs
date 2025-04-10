use std::collections::HashMap;

use common_lang_types::{
    ClientScalarSelectableName, ConstExportName, IsographDirectiveName, IsographObjectTypeName,
    Location, ObjectTypeAndFieldName, RelativePathToSourceFile, SelectableName,
    ServerObjectSelectableName, TextSource, UnvalidatedTypeName, VariableName, WithLocation,
    WithSpan,
};
use intern::string_key::Intern;
use isograph_lang_types::{
    ArgumentKeyAndValue, ClientFieldDeclaration, ClientObjectSelectableId,
    ClientPointerDeclaration, ClientScalarSelectableId, DefinitionLocation, DeserializationError,
    NonConstantValue, ObjectSelectionDirectiveSet, ScalarSelectionDirectiveSet, SelectionType,
    SelectionTypeContainingSelections, ServerEntityId, ServerObjectEntityId, TypeAnnotation,
    VariableDefinition,
};
use lazy_static::lazy_static;

use thiserror::Error;

use crate::{
    expose_field_directive::RequiresRefinement,
    refetch_strategy::{generate_refetch_field_strategy, id_selection, RefetchStrategy},
    ClientObjectSelectable, ClientScalarSelectable, FieldMapItem, NetworkProtocol, Schema,
    NODE_FIELD_NAME,
};

pub type UnprocessedSelection = WithSpan<
    SelectionTypeContainingSelections<ScalarSelectionDirectiveSet, ObjectSelectionDirectiveSet>,
>;

pub struct UnprocessedClientFieldItem {
    pub client_field_id: ClientScalarSelectableId,
    pub reader_selection_set: Vec<UnprocessedSelection>,
    pub refetch_strategy:
        Option<RefetchStrategy<ScalarSelectionDirectiveSet, ObjectSelectionDirectiveSet>>,
}
pub struct UnprocessedClientPointerItem {
    pub client_pointer_id: ClientObjectSelectableId,
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
            .get(&client_field_declaration.item.parent_type.item)
            .ok_or(WithLocation::new(
                ProcessClientFieldDeclarationError::ParentTypeNotDefined {
                    parent_type_name: client_field_declaration.item.parent_type.item,
                },
                Location::new(text_source, client_field_declaration.item.parent_type.span),
            ))?;

        let unprocess_client_field_items = match parent_type_id {
            ServerEntityId::Object(object_entity_id) => self
                .add_client_field_to_object(*object_entity_id, client_field_declaration)
                .map_err(|e| WithLocation::new(e.item, Location::new(text_source, e.span)))?,
            ServerEntityId::Scalar(scalar_entity_id) => {
                let scalar_name = self
                    .server_entity_data
                    .server_scalar_entity(*scalar_entity_id)
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
            .server_entity_data
            .defined_entities
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

        let unprocessed_client_pointer_items = match parent_type_id {
            ServerEntityId::Object(object_entity_id) => match target_type_id {
                ServerEntityId::Object(to_object_entity_id) => self
                    .add_client_pointer_to_object(
                        *object_entity_id,
                        TypeAnnotation::from_graphql_type_annotation(
                            client_pointer_declaration
                                .item
                                .target_type
                                .clone()
                                .map(|_| *to_object_entity_id),
                        ),
                        client_pointer_declaration,
                    )
                    .map_err(|e| WithLocation::new(e.item, Location::new(text_source, e.span)))?,
                ServerEntityId::Scalar(scalar_entity_id) => {
                    let scalar_name = self
                        .server_entity_data
                        .server_scalar_entity(*scalar_entity_id)
                        .name;
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
            ServerEntityId::Scalar(scalar_entity_id) => {
                let scalar_name = self
                    .server_entity_data
                    .server_scalar_entity(*scalar_entity_id)
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
        parent_object_entity_id: ServerObjectEntityId,
        client_field_declaration: WithSpan<ClientFieldDeclaration>,
    ) -> ProcessClientFieldDeclarationResult<UnprocessedClientFieldItem> {
        let query_id = self.query_id();
        let object =
            &mut self.server_entity_data.server_objects[parent_object_entity_id.as_usize()];
        let client_field_field_name_ws = client_field_declaration.item.client_field_name;
        let client_field_name = client_field_field_name_ws.item;
        let client_field_name_span = client_field_field_name_ws.span;

        let next_client_field_id = self.client_scalar_selectables.len().into();

        if self
            .server_entity_data
            .server_object_entity_available_selectables
            .entry(parent_object_entity_id)
            .or_default()
            .0
            .insert(
                client_field_name.into(),
                DefinitionLocation::Client(SelectionType::Scalar(next_client_field_id)),
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

        let name = client_field_declaration.item.client_field_name.item;
        let variant = get_client_variant(&client_field_declaration.item);

        self.client_scalar_selectables.push(ClientScalarSelectable {
            description: client_field_declaration.item.description.map(|x| x.item),
            name,
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
                        client_field_name.into(),
                    )
                })
                .collect::<Result<_, _>>()?,
            type_and_field: ObjectTypeAndFieldName {
                type_name: object.name,
                field_name: name.into(),
            },

            parent_object_entity_id,
            refetch_strategy: None,
            output_format: std::marker::PhantomData,
        });

        let selections = client_field_declaration.item.selection_set;
        let id_field = self
            .server_entity_data
            .server_object_entity_available_selectables
            .get(&parent_object_entity_id)
            .expect(
                "Expected parent_object_entity_id to exist in \
                server_object_entity_available_selectables",
            )
            .1;
        let refetch_strategy = id_field.map(|_| {
            // Assume that if we have an id field, this implements Node
            RefetchStrategy::UseRefetchField(generate_refetch_field_strategy(
                vec![id_selection()],
                query_id,
                format!("refetch__{}", object.name).intern().into(),
                *NODE_FIELD_NAME,
                id_top_level_arguments(),
                None,
                RequiresRefinement::Yes(object.name),
                vec![],
                None,
            ))
        });

        Ok(UnprocessedClientFieldItem {
            client_field_id: next_client_field_id,
            reader_selection_set: selections,
            refetch_strategy,
        })
    }

    fn add_client_pointer_to_object(
        &mut self,
        parent_object_entity_id: ServerObjectEntityId,
        to_object_entity_id: TypeAnnotation<ServerObjectEntityId>,
        client_pointer_declaration: WithSpan<ClientPointerDeclaration>,
    ) -> ProcessClientFieldDeclarationResult<UnprocessedClientPointerItem> {
        let query_id = self.query_id();
        let to_object = self
            .server_entity_data
            .server_object_entity(*to_object_entity_id.inner());
        let parent_object = self
            .server_entity_data
            .server_object_entity(parent_object_entity_id);
        let client_pointer_pointer_name_ws = client_pointer_declaration.item.client_pointer_name;
        let client_pointer_name = client_pointer_pointer_name_ws.item;
        let client_pointer_name_span = client_pointer_pointer_name_ws.span;

        let next_client_pointer_id: ClientObjectSelectableId =
            self.client_object_selectables.len().into();

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

        let unprocessed_fields = client_pointer_declaration.item.selection_set;

        let id_field = self
            .server_entity_data
            .server_object_entity_available_selectables
            .get(&parent_object_entity_id)
            .expect(
                "Expected parent_object_entity_id \
                to exist in server_object_entity_available_selectables",
            )
            .1;
        let refetch_strategy = match id_field {
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
                        vec![],
                        query_id,
                        format!("refetch__{}", to_object.name).intern().into(),
                        *NODE_FIELD_NAME,
                        id_top_level_arguments(),
                        None,
                        RequiresRefinement::Yes(to_object.name),
                        vec![],
                        None,
                    ),
                ))
            }
        }?;

        self.client_object_selectables.push(ClientObjectSelectable {
            description: client_pointer_declaration.item.description.map(|x| x.item),
            name,
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
                        client_pointer_name.into(),
                    )
                })
                .collect::<Result<_, _>>()?,
            type_and_field: ObjectTypeAndFieldName {
                type_name: parent_object.name,
                field_name: name.into(),
            },

            parent_object_entity_id,
            refetch_strategy,
            to: to_object_entity_id,
            output_format: std::marker::PhantomData,

            info: UserWrittenClientPointerInfo {
                const_export_name: client_pointer_declaration.item.const_export_name,
                file_path: client_pointer_declaration.item.definition_path,
            },
        });

        if self
            .server_entity_data
            .server_object_entity_available_selectables
            .entry(parent_object_entity_id)
            .or_default()
            .0
            .insert(
                client_pointer_name.into(),
                DefinitionLocation::Client(SelectionType::Object(next_client_pointer_id)),
            )
            .is_some()
        {
            let parent_object = self
                .server_entity_data
                .server_object_entity(parent_object_entity_id);
            // Did not insert, so this object already has a field with the same name :(
            return Err(WithSpan::new(
                ProcessClientFieldDeclarationError::ParentAlreadyHasField {
                    parent_type_name: parent_object.name,
                    client_field_name: client_pointer_name.into(),
                },
                client_pointer_name_span,
            ));
        }

        Ok(UnprocessedClientPointerItem {
            client_pointer_id: next_client_pointer_id,
            reader_selection_set: unprocessed_fields,
            refetch_selection_set: vec![id_selection()],
        })
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
        client_field_name: SelectableName,
    },

    #[error("Error when deserializing directives. Message: {message}")]
    UnableToDeserializeDirectives { message: DeserializationError },

    #[error(
        "The argument `{argument_name}` on field `{parent_type_name}.{field_name}` has inner type `{argument_type}`, which does not exist."
    )]
    FieldArgumentTypeDoesNotExist {
        argument_name: VariableName,
        parent_type_name: IsographObjectTypeName,
        field_name: SelectableName,
        argument_type: UnvalidatedTypeName,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PrimaryFieldInfo {
    pub primary_field_name: ServerObjectSelectableName,
    /// Some if the object is concrete; None otherwise.
    pub primary_field_concrete_type: Option<IsographObjectTypeName>,
    /// If this is abstract, we add a fragment spread
    pub primary_field_return_type_object_entity_id: ServerObjectEntityId,
    pub primary_field_field_map: Vec<FieldMapItem>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ImperativelyLoadedFieldVariant {
    pub client_field_scalar_selection_name: ClientScalarSelectableName,
    /// What field should we select when generating the refetch query?
    pub top_level_schema_field_name: ServerObjectSelectableName,
    /// The arguments we must pass to the top level schema field, e.g. id: ID!
    /// for node(id: $id)
    pub top_level_schema_field_arguments: Vec<VariableDefinition<ServerEntityId>>,

    /// Some if the object is concrete; None otherwise.
    pub top_level_schema_field_concrete_type: Option<IsographObjectTypeName>,

    /// If we need to select a sub-field, this is Some(...). We should model
    /// this differently, this is very awkward!
    pub primary_field_info: Option<PrimaryFieldInfo>,

    pub root_object_entity_id: ServerObjectEntityId,
}

// TODO Component is a GraphQL-ism
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum UserWrittenComponentVariant {
    Eager,
    Component,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UserWrittenClientTypeInfo {
    // TODO use a shared struct
    pub const_export_name: ConstExportName,
    pub file_path: RelativePathToSourceFile,
    pub user_written_component_variant: UserWrittenComponentVariant,
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

lazy_static! {
    static ref COMPONENT: IsographDirectiveName = "component".intern().into();
}

fn get_client_variant(client_field_declaration: &ClientFieldDeclaration) -> ClientFieldVariant {
    for directive in client_field_declaration.directives.iter() {
        if directive.item.name.item == *COMPONENT {
            return ClientFieldVariant::UserWritten(UserWrittenClientTypeInfo {
                const_export_name: client_field_declaration.const_export_name,
                file_path: client_field_declaration.definition_path,
                user_written_component_variant: UserWrittenComponentVariant::Component,
            });
        }
    }
    ClientFieldVariant::UserWritten(UserWrittenClientTypeInfo {
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

pub fn validate_variable_definition(
    defined_types: &HashMap<UnvalidatedTypeName, ServerEntityId>,
    variable_definition: WithSpan<VariableDefinition<UnvalidatedTypeName>>,
    parent_type_name: IsographObjectTypeName,
    field_name: SelectableName,
) -> ProcessClientFieldDeclarationResult<WithSpan<VariableDefinition<ServerEntityId>>> {
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
