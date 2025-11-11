use std::ops::Deref;

use common_lang_types::{
    ClientObjectSelectableName, ClientScalarSelectableName, ClientSelectableName, ConstExportName,
    IsographDirectiveName, Location, ParentObjectEntityNameAndSelectableName,
    RelativePathToSourceFile, SelectableName, ServerObjectEntityName, Span, TextSource,
    UnvalidatedTypeName, VariableName, WithLocation, WithSpan,
};
use intern::string_key::Intern;
use isograph_lang_types::{
    ArgumentKeyAndValue, ClientFieldDeclaration, ClientFieldDirectiveSet, ClientPointerDeclaration,
    DefinitionLocation, DeserializationError, NonConstantValue, SelectionType,
    ServerObjectEntityNameWrapper, TypeAnnotation, UnvalidatedSelection, VariableDefinition,
};

use pico_macros::legacy_memo;
use thiserror::Error;

use crate::{
    ClientObjectSelectable, ClientScalarSelectable, FieldMapItem,
    FieldToInsertToServerSelectableError, ID_FIELD_NAME, IsographDatabase, NODE_FIELD_NAME,
    NetworkProtocol, Schema, ServerEntityName, ServerSelectableNamedError,
    ValidatedVariableDefinition, WrappedSelectionMapSelection, defined_entity, fetchable_types,
    refetch_strategy::{RefetchStrategy, generate_refetch_field_strategy, id_selection},
    server_selectable_named,
};

pub type UnprocessedSelection = WithSpan<UnvalidatedSelection>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnprocessedClientScalarSelectableSelectionSet {
    pub parent_object_entity_name: ServerObjectEntityName,
    pub client_scalar_selectable_name: ClientScalarSelectableName,
    pub reader_selection_set: Vec<UnprocessedSelection>,
    pub refetch_strategy: Option<RefetchStrategy<(), ()>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnprocessedClientObjectSelectableSelectionSet {
    pub parent_object_entity_name: ServerObjectEntityName,
    pub client_object_selectable_name: ClientObjectSelectableName,
    pub reader_selection_set: Vec<UnprocessedSelection>,
    pub refetch_selection_set: Vec<UnprocessedSelection>,
}
pub type UnprocessedSelectionSet = SelectionType<
    UnprocessedClientScalarSelectableSelectionSet,
    UnprocessedClientObjectSelectableSelectionSet,
>;

pub fn process_client_field_declaration<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    schema: &mut Schema<TNetworkProtocol>,
    client_field_declaration: WithSpan<ClientFieldDeclaration>,
    text_source: TextSource,
) -> Result<
    UnprocessedClientScalarSelectableSelectionSet,
    WithLocation<ProcessClientFieldDeclarationError<TNetworkProtocol>>,
> {
    let parent_type_id =
        defined_entity(db, client_field_declaration.item.parent_type.item.0.into())
            .to_owned()
            .expect(
                "Expected parsing to have succeeded. \
                This is indicative of a bug in Isograph.",
            )
            .ok_or(WithLocation::new(
                ProcessClientFieldDeclarationError::ParentTypeNotDefined {
                    parent_object_entity_name: client_field_declaration.item.parent_type.item,
                },
                Location::new(text_source, client_field_declaration.item.parent_type.span),
            ))?;

    let unprocess_client_field_items = match parent_type_id {
        ServerEntityName::Object(_) => {
            add_client_field_to_object(db, schema, client_field_declaration)
                .map_err(|e| WithLocation::new(e.item, Location::new(text_source, e.span)))?
        }
        ServerEntityName::Scalar(scalar_entity_name) => {
            return Err(WithLocation::new(
                ProcessClientFieldDeclarationError::InvalidParentType {
                    literal_type: "field",
                    parent_object_entity_name: scalar_entity_name.into(),
                },
                Location::new(text_source, client_field_declaration.item.parent_type.span),
            ));
        }
    };

    Ok(unprocess_client_field_items)
}

pub fn process_client_pointer_declaration<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    schema: &mut Schema<TNetworkProtocol>,
    client_pointer_declaration: WithSpan<ClientPointerDeclaration>,
    text_source: TextSource,
) -> Result<
    UnprocessedClientObjectSelectableSelectionSet,
    WithLocation<ProcessClientFieldDeclarationError<TNetworkProtocol>>,
> {
    let parent_type_id = defined_entity(
        db,
        client_pointer_declaration.item.parent_type.item.0.into(),
    )
    .to_owned()
    .expect(
        "Expected parsing to have succeeded. \
        This is indicative of a bug in Isograph.",
    )
    .ok_or(WithLocation::new(
        ProcessClientFieldDeclarationError::ParentTypeNotDefined {
            parent_object_entity_name: client_pointer_declaration.item.parent_type.item,
        },
        Location::new(
            text_source,
            client_pointer_declaration.item.parent_type.span,
        ),
    ))?;

    let target_type_id = defined_entity(
        db,
        client_pointer_declaration.item.target_type.inner().0.into(),
    )
    .to_owned()
    .expect(
        "Expected parsing to have succeeded. \
            This is indicative of a bug in Isograph.",
    )
    .ok_or(WithLocation::new(
        ProcessClientFieldDeclarationError::ParentTypeNotDefined {
            parent_object_entity_name: *client_pointer_declaration.item.target_type.inner(),
        },
        Location::new(
            text_source,
            client_pointer_declaration.item.target_type.span(),
        ),
    ))?;

    let unprocessed_client_object_selection_set = match parent_type_id {
        ServerEntityName::Object(object_entity_name) => match target_type_id {
            ServerEntityName::Object(_to_object_entity_name) => add_client_pointer_to_object(
                db,
                schema,
                object_entity_name,
                client_pointer_declaration,
            )
            .map_err(|e| WithLocation::new(e.item, Location::new(text_source, e.span)))?,
            ServerEntityName::Scalar(scalar_entity_name) => {
                return Err(WithLocation::new(
                    ProcessClientFieldDeclarationError::ClientPointerInvalidTargetType {
                        target_object_entity_name: scalar_entity_name.into(),
                    },
                    Location::new(
                        text_source,
                        client_pointer_declaration.item.target_type.span(),
                    ),
                ));
            }
        },
        ServerEntityName::Scalar(scalar_entity_name) => {
            return Err(WithLocation::new(
                ProcessClientFieldDeclarationError::InvalidParentType {
                    literal_type: "pointer",
                    parent_object_entity_name: scalar_entity_name.into(),
                },
                Location::new(
                    text_source,
                    client_pointer_declaration.item.parent_type.span,
                ),
            ));
        }
    };
    Ok(unprocessed_client_object_selection_set)
}

fn add_client_field_to_object<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    schema: &mut Schema<TNetworkProtocol>,
    client_field_declaration: WithSpan<ClientFieldDeclaration>,
) -> ProcessClientFieldDeclarationResult<
    UnprocessedClientScalarSelectableSelectionSet,
    TNetworkProtocol,
> {
    let name_span = client_field_declaration
        .item
        .client_field_name
        .location
        .span;
    let client_scalar_selectable_name = client_field_declaration.item.client_field_name.item;
    let client_field_parent_object_entity_name = client_field_declaration.item.parent_type.item.0;

    let (result, client_scalar_selectable) =
        process_client_field_declaration_inner(db, client_field_declaration).to_owned()?;

    if schema
        .server_entity_data
        .entry(client_field_parent_object_entity_name)
        .or_default()
        .selectables
        .insert(
            client_scalar_selectable_name.0.into(),
            DefinitionLocation::Client(SelectionType::Scalar((
                client_field_parent_object_entity_name,
                client_scalar_selectable_name.0,
            ))),
        )
        .is_some()
    {
        // Did not insert, so this object already has a field with the same name :(
        return Err(WithSpan::new(
            ProcessClientFieldDeclarationError::ParentAlreadyHasField {
                parent_object_entity_name: client_field_parent_object_entity_name,
                client_selectable_name: client_scalar_selectable_name.0.into(),
            },
            name_span,
        ));
    }

    schema.client_scalar_selectables.insert(
        (
            client_field_parent_object_entity_name,
            client_scalar_selectable_name.0,
        ),
        client_scalar_selectable,
    );

    Ok(result)
}

#[legacy_memo]
pub fn process_client_field_declaration_inner<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    client_field_declaration: WithSpan<ClientFieldDeclaration>,
) -> ProcessClientFieldDeclarationResult<
    (
        UnprocessedClientScalarSelectableSelectionSet,
        ClientScalarSelectable<TNetworkProtocol>,
    ),
    TNetworkProtocol,
> {
    let query_id_memo_ref = fetchable_types(db);

    let query_id = query_id_memo_ref
        .deref()
        .as_ref()
        .map_err(|e| {
            WithSpan::new(
                ProcessClientFieldDeclarationError::ParseTypeSystemDocumentsError(e.clone()),
                Span::todo_generated(),
            )
        })?
        .iter()
        .find(|(_, root_operation_name)| root_operation_name.0 == "query")
        .expect("Expected query to be found")
        .0;

    let client_scalar_selectable_name = client_field_declaration.item.client_field_name.item;

    let variant = get_client_variant(&client_field_declaration.item);

    let selectable = ClientScalarSelectable {
        description: client_field_declaration.item.description.map(|x| x.item),
        name: client_field_declaration
            .item
            .client_field_name
            .map(|client_scalar_selectable_name| *client_scalar_selectable_name)
            .into_with_location(),
        reader_selection_set: vec![],
        variant,
        variable_definitions: client_field_declaration
            .item
            .variable_definitions
            .into_iter()
            .map(|variable_definition| {
                validate_variable_definition(
                    db,
                    variable_definition,
                    client_field_declaration.item.parent_type.item.0,
                    client_scalar_selectable_name.0.into(),
                )
            })
            .collect::<Result<_, _>>()?,
        type_and_field: ParentObjectEntityNameAndSelectableName {
            parent_object_entity_name: client_field_declaration.item.parent_type.item.0,
            selectable_name: client_scalar_selectable_name.0.into(),
        },

        parent_object_entity_name: client_field_declaration.item.parent_type.item.0,
        refetch_strategy: None,
        network_protocol: std::marker::PhantomData,
    };

    let selections = client_field_declaration.item.selection_set;

    let is_fetchable = fetchable_types(db)
        .deref()
        .as_ref()
        .expect(
            "Expected parsing to have succeeded. \
            This is indicative of a bug in Isograph.",
        )
        .contains_key(&client_field_declaration.item.parent_type.item.0);

    let refetch_strategy = if is_fetchable {
        Some(RefetchStrategy::RefetchFromRoot)
    } else {
        let id_field_memo_ref = server_selectable_named(
            db,
            client_field_declaration.item.parent_type.item.0,
            (*ID_FIELD_NAME).into(),
        );

        let id_field = id_field_memo_ref
            // TODO don't call to_owned
            .to_owned()
            .map_err(|e| {
                WithSpan::new(
                    ProcessClientFieldDeclarationError::ServerSelectableNamedError(e),
                    Span::todo_generated(),
                )
            })?
            .transpose()
            .map_err(|e| {
                WithSpan::new(
                    ProcessClientFieldDeclarationError::FieldToInsertToServerSelectableError(e),
                    Span::todo_generated(),
                )
            })?;

        id_field.map(|_| {
            // Assume that if we have an id field, this implements Node
            RefetchStrategy::UseRefetchField(generate_refetch_field_strategy(
                vec![id_selection()],
                *query_id,
                vec![
                    WrappedSelectionMapSelection::InlineFragment(
                        client_field_declaration.item.parent_type.item.0,
                    ),
                    WrappedSelectionMapSelection::LinkedField {
                        server_object_selectable_name: *NODE_FIELD_NAME,
                        arguments: id_top_level_arguments(),
                        concrete_type: None,
                    },
                ],
            ))
        })
    };

    Ok((
        UnprocessedClientScalarSelectableSelectionSet {
            parent_object_entity_name: client_field_declaration.item.parent_type.item.0,
            client_scalar_selectable_name: *client_scalar_selectable_name,
            reader_selection_set: selections,
            refetch_strategy,
        },
        selectable,
    ))
}

fn add_client_pointer_to_object<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    schema: &mut Schema<TNetworkProtocol>,
    parent_object_entity_name: ServerObjectEntityName,
    client_pointer_declaration: WithSpan<ClientPointerDeclaration>,
) -> ProcessClientFieldDeclarationResult<
    UnprocessedClientObjectSelectableSelectionSet,
    TNetworkProtocol,
> {
    let client_pointer_pointer_name_ws = client_pointer_declaration.item.client_pointer_name;
    let client_pointer_name = client_pointer_pointer_name_ws.item.0;
    let client_pointer_name_span = client_pointer_pointer_name_ws.location.span;

    if schema
        .server_entity_data
        .entry(parent_object_entity_name)
        .or_default()
        .selectables
        .insert(
            client_pointer_name.into(),
            DefinitionLocation::Client(SelectionType::Object((
                parent_object_entity_name,
                client_pointer_declaration.item.client_pointer_name.item.0,
            ))),
        )
        .is_some()
    {
        // Did not insert, so this object already has a field with the same name :(
        return Err(WithSpan::new(
            ProcessClientFieldDeclarationError::ParentAlreadyHasField {
                parent_object_entity_name,
                client_selectable_name: client_pointer_declaration
                    .item
                    .client_pointer_name
                    .item
                    .0
                    .into(),
            },
            client_pointer_name_span,
        ));
    }

    let (unprocessed_fields, client_object_selectable) =
        process_client_pointer_declaration_inner(db, client_pointer_declaration).to_owned()?;

    schema.client_object_selectables.insert(
        (parent_object_entity_name, client_pointer_name),
        client_object_selectable,
    );

    Ok(unprocessed_fields)
}

pub fn process_client_pointer_declaration_inner<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    client_pointer_declaration: WithSpan<ClientPointerDeclaration>,
) -> ProcessClientFieldDeclarationResult<
    (
        UnprocessedClientObjectSelectableSelectionSet,
        ClientObjectSelectable<TNetworkProtocol>,
    ),
    TNetworkProtocol,
> {
    let parent_object_entity_name = client_pointer_declaration.item.parent_type.item.0;
    let to_object_entity_name = client_pointer_declaration.item.target_type.inner().0;
    let client_pointer_name = client_pointer_declaration.item.client_pointer_name.item.0;

    let query_id = *fetchable_types(db)
        .deref()
        .as_ref()
        .expect(
            "Expected parsing to have succeeded. \
            This is indicative of a bug in Isograph.",
        )
        .iter()
        .find(|(_, root_operation_name)| root_operation_name.0 == "query")
        .expect("Expected query to be found")
        .0;

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

    let is_fetchable = fetchable_types(db)
        .deref()
        .as_ref()
        .expect(
            "Expected parsing to have succeeded. \
            This is indicative of a bug in Isograph.",
        )
        .contains_key(&to_object_entity_name);

    // TODO extract this into a helper function, probably on TNetworkProtocol
    let refetch_strategy = if is_fetchable {
        RefetchStrategy::RefetchFromRoot
    } else {
        let id_field_memo_ref = server_selectable_named(
            db,
            client_pointer_declaration.item.target_type.inner().0,
            (*ID_FIELD_NAME).into(),
        );

        let id_field = id_field_memo_ref
            // TODO don't call to_owned
            .to_owned()
            .map_err(|e| {
                WithSpan::new(
                    ProcessClientFieldDeclarationError::ServerSelectableNamedError(e),
                    Span::todo_generated(),
                )
            })?
            .transpose()
            .map_err(|e| {
                WithSpan::new(
                    ProcessClientFieldDeclarationError::FieldToInsertToServerSelectableError(e),
                    Span::todo_generated(),
                )
            })?;

        match id_field {
            None => Err(WithSpan::new(
                ProcessClientFieldDeclarationError::ClientPointerTargetTypeHasNoId {
                    target_object_entity_name: *client_pointer_declaration.item.target_type.inner(),
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
                            WrappedSelectionMapSelection::InlineFragment(to_object_entity_name),
                            WrappedSelectionMapSelection::LinkedField {
                                server_object_selectable_name: *NODE_FIELD_NAME,
                                arguments: id_top_level_arguments(),
                                concrete_type: None,
                            },
                        ],
                    ),
                ))
            }
        }?
    };

    let client_object_selectable = ClientObjectSelectable {
        description: client_pointer_declaration.item.description.map(|x| x.item),
        name: client_pointer_declaration
            .item
            .client_pointer_name
            .map(|client_object_selectable_name| *client_object_selectable_name)
            .into_with_location(),
        reader_selection_set: vec![],

        variable_definitions: client_pointer_declaration
            .item
            .variable_definitions
            .into_iter()
            .map(|variable_definition| {
                validate_variable_definition(
                    db,
                    variable_definition,
                    parent_object_entity_name,
                    client_pointer_name.into(),
                )
            })
            .collect::<Result<_, _>>()?,
        type_and_field: ParentObjectEntityNameAndSelectableName {
            parent_object_entity_name,
            selectable_name: client_pointer_name.into(),
        },

        parent_object_entity_name,
        refetch_strategy,
        target_object_entity_name: TypeAnnotation::from_graphql_type_annotation(
            client_pointer_declaration
                .item
                .target_type
                .clone()
                .map(|x| x.0),
        ),
        network_protocol: std::marker::PhantomData,

        info: UserWrittenClientPointerInfo {
            const_export_name: client_pointer_declaration.item.const_export_name,
            file_path: client_pointer_declaration.item.definition_path,
        },
    };

    Ok((
        UnprocessedClientObjectSelectableSelectionSet {
            client_object_selectable_name: client_pointer_name,
            parent_object_entity_name,
            reader_selection_set: unprocessed_fields,
            refetch_selection_set: vec![id_selection()],
        },
        client_object_selectable,
    ))
}

type ProcessClientFieldDeclarationResult<T, TNetworkProtocol> =
    Result<T, WithSpan<ProcessClientFieldDeclarationError<TNetworkProtocol>>>;

#[derive(Error, Eq, PartialEq, Debug, Clone)]
pub enum ProcessClientFieldDeclarationError<TNetworkProtocol: NetworkProtocol> {
    #[error("`{parent_object_entity_name}` is not a type that has been defined.")]
    ParentTypeNotDefined {
        parent_object_entity_name: ServerObjectEntityNameWrapper,
    },

    #[error("Directive `@{directive_name}` is not supported on client pointers.")]
    DirectiveNotSupportedOnClientPointer {
        directive_name: IsographDirectiveName,
    },

    #[error(
        "Invalid parent type. `{parent_object_entity_name}` is a scalar. \
        You are attempting to define a {literal_type} on it. \
        In order to do so, the parent object must be an object, interface or union."
    )]
    InvalidParentType {
        literal_type: &'static str,
        parent_object_entity_name: UnvalidatedTypeName,
    },

    #[error(
        "Invalid client pointer target type. `{target_object_entity_name}` is a scalar. \
        You are attempting to define a pointer to it. \
        In order to do so, the type must be an object, interface or union."
    )]
    ClientPointerInvalidTargetType {
        target_object_entity_name: UnvalidatedTypeName,
    },

    #[error(
        "Invalid client pointer target type. `{target_object_entity_name}` has no id field. \
        You are attempting to define a pointer to it. \
        In order to do so, the target must be an object implementing the `Node` interface."
    )]
    ClientPointerTargetTypeHasNoId {
        target_object_entity_name: ServerObjectEntityNameWrapper,
    },

    #[error(
        "The Isograph object type `{parent_object_entity_name}` already has a field named `{client_selectable_name}`."
    )]
    ParentAlreadyHasField {
        parent_object_entity_name: ServerObjectEntityName,
        client_selectable_name: ClientSelectableName,
    },

    #[error("Error when deserializing directives. Message: {message}")]
    UnableToDeserializeDirectives { message: DeserializationError },

    #[error(
        "The argument `{argument_name}` on field `{parent_object_entity_name}.{selectable_name}` \
        has inner type `{argument_type}`, which does not exist."
    )]
    FieldArgumentTypeDoesNotExist {
        argument_name: VariableName,
        parent_object_entity_name: ServerObjectEntityName,
        selectable_name: SelectableName,
        argument_type: UnvalidatedTypeName,
    },

    #[error("{0}")]
    ServerSelectableNamedError(ServerSelectableNamedError<TNetworkProtocol>),

    #[error("{0}")]
    FieldToInsertToServerSelectableError(FieldToInsertToServerSelectableError),

    #[error("{0}")]
    ParseTypeSystemDocumentsError(TNetworkProtocol::ParseTypeSystemDocumentsError),
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ImperativelyLoadedFieldVariant {
    pub client_selection_name: ClientSelectableName,

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

pub fn validate_variable_definition<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    variable_definition: WithSpan<VariableDefinition<UnvalidatedTypeName>>,
    parent_object_entity_name: ServerObjectEntityName,
    selectable_name: SelectableName,
) -> ProcessClientFieldDeclarationResult<
    WithSpan<VariableDefinition<ServerEntityName>>,
    TNetworkProtocol,
> {
    let type_ = variable_definition
        .item
        .type_
        .clone()
        .and_then(|input_type_name| {
            defined_entity(db, *variable_definition.item.type_.inner())
                .deref()
                .to_owned()
                .expect(
                    "Expected parsing to have succeeded. \
                    This is indicative of a bug in Isograph.",
                )
                .ok_or_else(|| {
                    WithSpan::new(
                        ProcessClientFieldDeclarationError::FieldArgumentTypeDoesNotExist {
                            argument_type: input_type_name,
                            argument_name: variable_definition.item.name.item,
                            parent_object_entity_name,
                            selectable_name,
                        },
                        variable_definition.span,
                    )
                })
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
