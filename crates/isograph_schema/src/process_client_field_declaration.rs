use common_lang_types::{
    Diagnostic, DiagnosticResult, EmbeddedLocation, EntityName, Location, SelectableName,
    VariableName, WithEmbeddedLocation, WithGenericLocation, WithLocationPostfix,
};
use isograph_lang_types::{
    ArgumentKeyAndValue, ClientFieldDeclaration, ClientPointerDeclaration, NonConstantValue,
    SelectionSet, SelectionType,
};
use pico::MemoRef;
use prelude::Postfix;

use pico_macros::memo;

use crate::{
    ClientFieldVariant, ClientObjectSelectable, ClientScalarSelectable, CompilationProfile,
    ID_FIELD_NAME, IsoLiteralExportInfo, IsographDatabase, NetworkProtocol,
    UserWrittenClientTypeInfo, flattened_entity_named, flattened_selectable_named,
    refetch_strategy::{RefetchStrategy, generate_refetch_field_strategy, id_selection},
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
    pub parent_entity_name: EntityName,
    pub client_object_selectable_name: SelectableName,
    pub reader_selection_set: WithEmbeddedLocation<SelectionSet>,
    pub refetch_selection_set: WithEmbeddedLocation<SelectionSet>,
}
pub type UnprocessedSelectionSet = SelectionType<
    UnprocessedClientScalarSelectableSelectionSet,
    UnprocessedClientObjectSelectableSelectionSet,
>;

pub fn process_client_field_declaration<TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
    client_field_declaration: MemoRef<ClientFieldDeclaration>,
) -> DiagnosticResult<UnprocessedClientScalarSelectableSelectionSet> {
    let client_field_declaration_item = client_field_declaration.lookup(db);
    let parent_entity =
        flattened_entity_named(db, client_field_declaration_item.parent_type.item.0)
            .ok_or_else(|| {
                let parent_object_entity_name = client_field_declaration_item.parent_type.item;
                Diagnostic::new(
                    format!("`{parent_object_entity_name}` is not a type that has been defined."),
                    client_field_declaration_item
                        .parent_type
                        .location
                        .to::<Location>()
                        .wrap_some(),
                )
            })?
            .lookup(db);

    match parent_entity.selection_info {
        SelectionType::Object(_) => {
            add_client_scalar_selectable_to_entity(db, client_field_declaration)
                .clone()
                .note_todo("Do not clone. Use a MemoRef.")
                .map(|x| x.0)?
        }
        SelectionType::Scalar(_) => {
            let scalar_entity_name = client_field_declaration_item.parent_type.item.0;
            return Diagnostic::new(
                format!(
                    "Invalid parent type. `{scalar_entity_name}` is a scalar. \
                    You are attempting to define a client field on it. \
                    In order to do so, the parent object must \
                    be an object, interface or union."
                ),
                client_field_declaration_item
                    .parent_type
                    .location
                    .to::<Location>()
                    .wrap_some(),
            )
            .wrap_err();
        }
    }
    .wrap_ok()
}

pub fn process_client_pointer_declaration<TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
    client_pointer_declaration: MemoRef<ClientPointerDeclaration>,
) -> DiagnosticResult<UnprocessedClientObjectSelectableSelectionSet> {
    let client_pointer_declaration_item = client_pointer_declaration.lookup(db);
    let parent_entity =
        flattened_entity_named(db, client_pointer_declaration_item.parent_type.item.0)
            .ok_or_else(|| {
                let parent_object_entity_name = client_pointer_declaration_item.parent_type.item;
                Diagnostic::new(
                    format!("`{parent_object_entity_name}` is not a type that has been defined."),
                    client_pointer_declaration_item
                        .parent_type
                        .location
                        .to::<Location>()
                        .wrap_some(),
                )
            })?
            .lookup(db);

    let target_entity = flattened_entity_named(
        db,
        client_pointer_declaration_item.target_type.item.inner().0,
    )
    .ok_or_else(|| {
        let target_type = client_pointer_declaration_item.target_type.item.inner();
        Diagnostic::new(
            format!("`{target_type}` is not a type that has been defined."),
            client_pointer_declaration_item
                .target_type
                .location
                .to::<Location>()
                .wrap_some(),
        )
    })?
    .lookup(db);

    match parent_entity.selection_info {
        SelectionType::Object(_) => match target_entity.selection_info {
            SelectionType::Object(_to_object_entity_name) => {
                add_client_pointer_to_object(db, client_pointer_declaration)?
            }
            SelectionType::Scalar(_) => {
                let scalar_entity_name = target_entity.name;
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
                        .location
                        .to::<Location>()
                        .wrap_some(),
                )
                .wrap_err();
            }
        },
        SelectionType::Scalar(_) => {
            // What?? This just seems wrong.
            let scalar_entity_name = parent_entity.name;
            return Diagnostic::new(
                format!("`{scalar_entity_name}` is not a type that has been defined."),
                client_pointer_declaration_item
                    .target_type
                    .location
                    .to::<Location>()
                    .wrap_some(),
            )
            .wrap_err();
        }
    }
    .wrap_ok()
}

#[memo]
pub fn add_client_scalar_selectable_to_entity<TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
    client_field_declaration: MemoRef<ClientFieldDeclaration>,
) -> DiagnosticResult<(
    UnprocessedClientScalarSelectableSelectionSet,
    MemoRef<ClientScalarSelectable<TCompilationProfile>>,
)> {
    let client_field_declaration = client_field_declaration.lookup(db);
    let client_scalar_selectable_name = client_field_declaration.client_field_name.item;

    let variant = get_client_variant(client_field_declaration);

    let selectable = ClientScalarSelectable {
        description: client_field_declaration
            .description
            .map(WithGenericLocation::drop_location),
        name: client_field_declaration
            .client_field_name
            .map(|client_scalar_selectable_name| *client_scalar_selectable_name)
            .item,
        variant,
        arguments: client_field_declaration
            .variable_definitions
            .iter()
            .map(|x| x.item.clone())
            .collect(),
        parent_entity_name: client_field_declaration.parent_type.item.0,
        phantom_data: std::marker::PhantomData,
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

pub fn get_refetch_stategy<TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
    parent_object_entity_name: EntityName,
) -> DiagnosticResult<Option<RefetchStrategy>> {
    if let Ok(wrapped_selection_map) =
        TCompilationProfile::NetworkProtocol::wrap_merged_selection_map(
            db,
            parent_object_entity_name,
            Default::default(),
        )
        && wrapped_selection_map
            .merged_selection_map
            .inner()
            .is_empty()
    {
        return RefetchStrategy::RefetchFromRoot.wrap_some().wrap_ok();
    }

    let id_field = flattened_selectable_named(db, parent_object_entity_name, *ID_FIELD_NAME);

    id_field
        .map(|_| {
            RefetchStrategy::UseRefetchField(generate_refetch_field_strategy(
                SelectionSet {
                    selections: vec![id_selection()],
                }
                .with_location(EmbeddedLocation::todo_generated()),
                parent_object_entity_name,
                vec![],
            ))
        })
        .wrap_ok()
}

fn add_client_pointer_to_object<TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
    client_pointer_declaration: MemoRef<ClientPointerDeclaration>,
) -> DiagnosticResult<UnprocessedClientObjectSelectableSelectionSet> {
    let (unprocessed_fields, _) =
        process_client_pointer_declaration_inner(db, client_pointer_declaration).to_owned()?;

    unprocessed_fields.wrap_ok()
}

#[memo]
pub fn process_client_pointer_declaration_inner<TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
    client_pointer_declaration: MemoRef<ClientPointerDeclaration>,
) -> DiagnosticResult<(
    UnprocessedClientObjectSelectableSelectionSet,
    MemoRef<ClientObjectSelectable<TCompilationProfile>>,
)> {
    let client_pointer_declaration = client_pointer_declaration.lookup(db);
    let parent_entity_name = client_pointer_declaration.parent_type.item.0;
    let client_pointer_name = client_pointer_declaration.client_pointer_name.item.0;

    if let Some(directive) = client_pointer_declaration.directives.item.first() {
        let directive_name = directive.item.name.item;
        return Diagnostic::new(
            format!("Directive `@{directive_name}` is not supported on client pointers."),
            client_pointer_declaration
                .client_pointer_name
                .location
                .to::<Location>()
                .wrap_some(),
        )
        .wrap_err();
    }

    let unprocessed_fields = client_pointer_declaration.selection_set.clone();

    let client_object_selectable = ClientObjectSelectable {
        description: client_pointer_declaration
            .description
            .map(WithGenericLocation::drop_location),
        name: client_pointer_declaration
            .client_pointer_name
            .map(|client_object_selectable_name| *client_object_selectable_name)
            .item,

        arguments: client_pointer_declaration
            .variable_definitions
            .iter()
            .map(|variable_definition| variable_definition.item.clone())
            .collect(),

        parent_entity_name,
        target_entity: client_pointer_declaration.target_type.item.clone(),
        phantom_data: std::marker::PhantomData,

        variant: ClientFieldVariant::UserWritten(UserWrittenClientTypeInfo {
            info: IsoLiteralExportInfo {
                const_export_name: client_pointer_declaration.const_export_name,
                file_path: client_pointer_declaration.definition_path,
            },
            directive_set: client_pointer_declaration.directives.clone(),
        }),
    };

    Ok((
        UnprocessedClientObjectSelectableSelectionSet {
            client_object_selectable_name: client_pointer_name,
            parent_entity_name,
            reader_selection_set: unprocessed_fields,
            refetch_selection_set: SelectionSet {
                selections: vec![id_selection()],
            }
            .with_location(EmbeddedLocation::todo_generated()),
        },
        client_object_selectable.interned_value(db),
    ))
}

fn get_client_variant(client_field_declaration: &ClientFieldDeclaration) -> ClientFieldVariant {
    ClientFieldVariant::UserWritten(UserWrittenClientTypeInfo {
        info: IsoLiteralExportInfo {
            const_export_name: client_field_declaration.const_export_name,
            file_path: client_field_declaration.definition_path,
        },
        directive_set: client_field_declaration.directive_set.clone(),
    })
}

pub fn id_top_level_arguments() -> Vec<ArgumentKeyAndValue> {
    vec![ArgumentKeyAndValue {
        key: ID_FIELD_NAME.unchecked_conversion(),
        value: NonConstantValue::Variable(
            ID_FIELD_NAME.unchecked_conversion::<VariableName>().into(),
        ),
    }]
}
