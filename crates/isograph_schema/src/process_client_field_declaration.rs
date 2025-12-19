use common_lang_types::{
    ConstExportName, Diagnostic, DiagnosticResult, EmbeddedLocation, EntityName, Location,
    RelativePathToSourceFile, SelectableName, VariableName, WithEmbeddedLocation,
    WithLocationPostfix,
};
use isograph_lang_types::{
    ArgumentKeyAndValue, ClientFieldDeclaration, ClientPointerDeclaration,
    ClientScalarSelectableDirectiveSet, NonConstantValue, SelectionSet, SelectionType,
    TypeAnnotation, VariableDefinition,
};
use pico::MemoRef;
use prelude::{ErrClone, Postfix};

use pico_macros::memo;

use crate::{
    ClientObjectSelectable, ClientScalarSelectable, FieldMapItem, ID_FIELD_NAME, IsographDatabase,
    NODE_FIELD_NAME, NetworkProtocol, ServerEntityName, ValidatedVariableDefinition,
    WrappedSelectionMapSelection, defined_entity, fetchable_types,
    refetch_strategy::{RefetchStrategy, generate_refetch_field_strategy, id_selection},
    server_selectable_named,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnprocessedClientScalarSelectableSelectionSet {
    pub parent_object_entity_name: EntityName,
    pub client_scalar_selectable_name: SelectableName,
    pub reader_selection_set: WithEmbeddedLocation<SelectionSet>,
    pub refetch_strategy: Option<RefetchStrategy>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnprocessedClientObjectSelectableSelectionSet {
    pub parent_object_entity_name: EntityName,
    pub client_object_selectable_name: SelectableName,
    pub reader_selection_set: WithEmbeddedLocation<SelectionSet>,
    pub refetch_selection_set: WithEmbeddedLocation<SelectionSet>,
}
pub type UnprocessedSelectionSet = SelectionType<
    UnprocessedClientScalarSelectableSelectionSet,
    UnprocessedClientObjectSelectableSelectionSet,
>;

pub fn process_client_field_declaration<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    client_field_declaration: MemoRef<ClientFieldDeclaration>,
) -> DiagnosticResult<UnprocessedClientScalarSelectableSelectionSet> {
    let client_field_declaration_item = client_field_declaration.lookup(db);
    let parent_type_id = defined_entity(db, client_field_declaration_item.parent_type.item.0)
        .to_owned()?
        .ok_or_else(|| {
            let parent_object_entity_name = client_field_declaration_item.parent_type.item;
            Diagnostic::new(
                format!("`{parent_object_entity_name}` is not a type that has been defined."),
                client_field_declaration_item
                    .parent_type
                    .embedded_location
                    .to::<Location>()
                    .wrap_some(),
            )
        })?;

    match parent_type_id {
        ServerEntityName::Object(_) => {
            add_client_scalar_selectable_to_entity(db, client_field_declaration)
                .clone()
                .note_todo("Do not clone. Use a MemoRef.")
                .map(|x| x.0)?
        }
        ServerEntityName::Scalar(scalar_entity_name) => {
            return Diagnostic::new(
                format!(
                    "Invalid parent type. `{scalar_entity_name}` is a scalar. \
                    You are attempting to define a client field on it. \
                    In order to do so, the parent object must \
                    be an object, interface or union."
                ),
                client_field_declaration_item
                    .parent_type
                    .embedded_location
                    .to::<Location>()
                    .wrap_some(),
            )
            .wrap_err();
        }
    }
    .wrap_ok()
}

pub fn process_client_pointer_declaration<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    client_pointer_declaration: MemoRef<ClientPointerDeclaration>,
) -> DiagnosticResult<UnprocessedClientObjectSelectableSelectionSet> {
    let client_pointer_declaration_item = client_pointer_declaration.lookup(db);
    let parent_type_id = defined_entity(db, client_pointer_declaration_item.parent_type.item.0)
        .to_owned()?
        .ok_or_else(|| {
            let parent_object_entity_name = client_pointer_declaration_item.parent_type.item;
            Diagnostic::new(
                format!("`{parent_object_entity_name}` is not a type that has been defined."),
                client_pointer_declaration_item
                    .parent_type
                    .embedded_location
                    .to::<Location>()
                    .wrap_some(),
            )
        })?;

    let target_type_id = defined_entity(db, client_pointer_declaration_item.target_type.inner().0)
        .to_owned()?
        .ok_or_else(|| {
            let target_type = client_pointer_declaration_item.target_type.inner();
            Diagnostic::new(
                format!("`{target_type}` is not a type that has been defined."),
                client_pointer_declaration_item
                    .target_type
                    .embedded_location()
                    .to::<Location>()
                    .wrap_some(),
            )
        })?;

    match parent_type_id {
        ServerEntityName::Object(_) => match target_type_id {
            ServerEntityName::Object(_to_object_entity_name) => {
                add_client_pointer_to_object(db, client_pointer_declaration)?
            }
            ServerEntityName::Scalar(scalar_entity_name) => {
                return Diagnostic::new(
                    format!(
                        "Invalid client pointer target type. \
                        `{scalar_entity_name}` is a scalar. \
                        You are attempting to define a pointer to it. \
                        In order to do so, the type must be \
                        an object, interface or union."
                    ),
                    client_pointer_declaration_item
                        .target_type
                        .embedded_location()
                        .to::<Location>()
                        .wrap_some(),
                )
                .wrap_err();
            }
        },
        ServerEntityName::Scalar(scalar_entity_name) => {
            return Diagnostic::new(
                format!("`{scalar_entity_name}` is not a type that has been defined."),
                client_pointer_declaration_item
                    .target_type
                    .embedded_location()
                    .to::<Location>()
                    .wrap_some(),
            )
            .wrap_err();
        }
    }
    .wrap_ok()
}

#[memo]
pub fn add_client_scalar_selectable_to_entity<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    client_field_declaration: MemoRef<ClientFieldDeclaration>,
) -> DiagnosticResult<(
    UnprocessedClientScalarSelectableSelectionSet,
    MemoRef<ClientScalarSelectable<TNetworkProtocol>>,
)> {
    let client_field_declaration = client_field_declaration.lookup(db);
    let client_scalar_selectable_name = client_field_declaration.client_field_name.item;

    let variant = get_client_variant(client_field_declaration);

    let selectable = ClientScalarSelectable {
        description: client_field_declaration.description.map(|x| x.item),
        name: client_field_declaration
            .client_field_name
            .map(|client_scalar_selectable_name| *client_scalar_selectable_name)
            .item,
        variant,
        variable_definitions: client_field_declaration
            .variable_definitions
            .iter()
            .map(|variable_definition| {
                validate_variable_definition(
                    db,
                    variable_definition,
                    client_field_declaration.parent_type.item.0,
                    client_scalar_selectable_name.0,
                )
                .map(|x| x.item)
            })
            .collect::<Result<_, _>>()?,

        parent_object_entity_name: client_field_declaration.parent_type.item.0,
        network_protocol: std::marker::PhantomData,
    };

    let selections = client_field_declaration.selection_set.clone();

    let parent_object_entity_name = client_field_declaration.parent_type.item.0;
    let refetch_strategy = get_refetch_stategy(db, parent_object_entity_name)?;

    (
        UnprocessedClientScalarSelectableSelectionSet {
            parent_object_entity_name: client_field_declaration.parent_type.item.0,
            client_scalar_selectable_name: *client_scalar_selectable_name,
            reader_selection_set: selections,
            refetch_strategy,
        },
        selectable.interned_value(db),
    )
        .wrap_ok()
}

pub fn get_refetch_stategy<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    parent_object_entity_name: EntityName,
) -> DiagnosticResult<Option<RefetchStrategy>> {
    let fetchable_types_map = fetchable_types(db).clone_err()?.lookup(db);

    let is_fetchable = fetchable_types_map.contains_key(&parent_object_entity_name);

    if is_fetchable {
        Some(RefetchStrategy::RefetchFromRoot)
    } else {
        let id_field =
            server_selectable_named(db, parent_object_entity_name, *ID_FIELD_NAME).clone_err()?;

        let query_id = fetchable_types_map
            .iter()
            .find(|(_, root_operation_name)| root_operation_name.0 == "query")
            .expect("Expected query to be found")
            .0;

        id_field.map(|_| {
            // Assume that if we have an id field, this implements Node
            RefetchStrategy::UseRefetchField(generate_refetch_field_strategy(
                SelectionSet {
                    selections: vec![id_selection()],
                }
                .with_embedded_location(EmbeddedLocation::todo_generated()),
                *query_id,
                vec![
                    WrappedSelectionMapSelection::InlineFragment(parent_object_entity_name),
                    WrappedSelectionMapSelection::LinkedField {
                        parent_object_entity_name: *query_id,
                        server_object_selectable_name: *NODE_FIELD_NAME,
                        arguments: id_top_level_arguments(),
                        concrete_target_entity_name: None,
                    },
                ],
            ))
        })
    }
    .wrap_ok()
}

fn add_client_pointer_to_object<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    client_pointer_declaration: MemoRef<ClientPointerDeclaration>,
) -> DiagnosticResult<UnprocessedClientObjectSelectableSelectionSet> {
    let (unprocessed_fields, _) =
        process_client_pointer_declaration_inner(db, client_pointer_declaration).to_owned()?;

    unprocessed_fields.wrap_ok()
}

#[memo]
pub fn process_client_pointer_declaration_inner<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    client_pointer_declaration: MemoRef<ClientPointerDeclaration>,
) -> DiagnosticResult<(
    UnprocessedClientObjectSelectableSelectionSet,
    MemoRef<ClientObjectSelectable<TNetworkProtocol>>,
)> {
    let client_pointer_declaration = client_pointer_declaration.lookup(db);
    let parent_object_entity_name = client_pointer_declaration.parent_type.item.0;
    let client_pointer_name = client_pointer_declaration.client_pointer_name.item.0;

    if let Some(directive) = client_pointer_declaration.directives.item.first() {
        let directive_name = directive.item.name.item;
        return Diagnostic::new(
            format!("Directive `@{directive_name}` is not supported on client pointers."),
            client_pointer_declaration
                .client_pointer_name
                .embedded_location
                .to::<Location>()
                .wrap_some(),
        )
        .wrap_err();
    }

    let unprocessed_fields = client_pointer_declaration.selection_set.clone();

    let client_object_selectable = ClientObjectSelectable {
        description: client_pointer_declaration.description.map(|x| x.item),
        name: client_pointer_declaration
            .client_pointer_name
            .map(|client_object_selectable_name| *client_object_selectable_name)
            .item,

        variable_definitions: client_pointer_declaration
            .variable_definitions
            .iter()
            .map(|variable_definition| {
                validate_variable_definition(
                    db,
                    variable_definition,
                    parent_object_entity_name,
                    client_pointer_name,
                )
                .map(|x| x.item)
            })
            .collect::<Result<_, _>>()?,

        parent_object_entity_name,
        target_entity_name: TypeAnnotation::from_graphql_type_annotation(
            client_pointer_declaration.target_type.clone().map(|x| x.0),
        ),
        network_protocol: std::marker::PhantomData,

        info: UserWrittenClientPointerInfo {
            const_export_name: client_pointer_declaration.const_export_name,
            file_path: client_pointer_declaration.definition_path,
        },
    };

    Ok((
        UnprocessedClientObjectSelectableSelectionSet {
            client_object_selectable_name: client_pointer_name,
            parent_object_entity_name,
            reader_selection_set: unprocessed_fields,
            refetch_selection_set: SelectionSet {
                selections: vec![id_selection()],
            }
            .with_embedded_location(EmbeddedLocation::todo_generated()),
        },
        client_object_selectable.interned_value(db),
    ))
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ImperativelyLoadedFieldVariant {
    pub client_selection_name: SelectableName,

    // Mutation or Query or whatnot. Awkward! A GraphQL-ism!
    pub root_object_entity_name: EntityName,
    pub subfields_or_inline_fragments: Vec<WrappedSelectionMapSelection>,
    pub field_map: Vec<FieldMapItem>,
    /// The arguments we must pass to the top level schema field, e.g. id: ID!
    /// for node(id: $id). These are already encoded in the subfields_or_inline_fragments,
    /// but we nonetheless need to put them into the query definition, and we need
    /// the variable's type, not just the variable.
    pub top_level_schema_field_arguments: Vec<ValidatedVariableDefinition>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UserWrittenClientTypeInfo {
    // TODO use a shared struct
    pub const_export_name: ConstExportName,
    pub file_path: RelativePathToSourceFile,
    pub client_scalar_selectable_directive_set:
        Result<ClientScalarSelectableDirectiveSet, Diagnostic>,
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
        client_scalar_selectable_directive_set: client_field_declaration
            .client_scalar_selectable_directive_set
            .clone(),
    })
}

pub fn id_top_level_arguments() -> Vec<ArgumentKeyAndValue> {
    vec![ArgumentKeyAndValue {
        key: ID_FIELD_NAME.unchecked_conversion(),
        value: NonConstantValue::Variable(ID_FIELD_NAME.unchecked_conversion()),
    }]
}

pub fn validate_variable_definition<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    variable_definition: &WithEmbeddedLocation<VariableDefinition<EntityName>>,
    parent_object_entity_name: EntityName,
    selectable_name: SelectableName,
) -> DiagnosticResult<WithEmbeddedLocation<VariableDefinition<ServerEntityName>>> {
    let type_ = variable_definition
        .item
        .type_
        .clone()
        .and_then(|input_type_name| {
            defined_entity(db, *variable_definition.item.type_.inner())
                .to_owned()?
                .ok_or_else(|| {
                    let argument_name = variable_definition.item.name.item;
                    Diagnostic::new(
                        format!(
                            "The argument `{argument_name}` on field \
                            `{parent_object_entity_name}.{selectable_name}` \
                            has inner type `{input_type_name}`, which does not exist."
                        ),
                        variable_definition
                            .embedded_location
                            .to::<Location>()
                            .wrap_some(),
                    )
                })
        })?;

    VariableDefinition {
        name: variable_definition.item.name.map(VariableName::from),
        type_,
        default_value: variable_definition.item.default_value.clone(),
    }
    .with_embedded_location(variable_definition.embedded_location)
    .wrap_ok()
}
