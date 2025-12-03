use crate::{
    IsographDatabase, NetworkProtocol, ObjectSelectableId, RefetchStrategy, ScalarSelectableId,
    UseRefetchFieldRefetchStrategy, ValidatedObjectSelection, ValidatedScalarSelection,
    ValidatedSelection, selectable_named,
};
use common_lang_types::{
    Diagnostic, DiagnosticResult, DiagnosticVecResult, Location,
    ParentObjectEntityNameAndSelectableName, SelectableName, ServerObjectEntityName, WithSpan,
    WithSpanPostfix,
};
use isograph_lang_types::{
    DefinitionLocation, DefinitionLocationPostfix, ObjectSelection, ScalarSelection,
    ScalarSelectionDirectiveSet, SelectionSet, SelectionType, SelectionTypePostfix,
    UnvalidatedScalarFieldSelection, UnvalidatedSelection,
};
use prelude::{ErrClone, Postfix};

/// At this point, all selectables have been defined. So, we can validate the parsed
/// selection set by confirming:
/// - each selectable exists in the schema, and is selected correctly (e.g. client fields
///   as scalars, etc)
/// - validate loadability/selectability (e.g. client fields cannot be selected updatably), and
/// - include the selectable id in the associated data
pub fn get_validated_selection_set<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    selection_set: WithSpan<SelectionSet<(), ()>>,
    parent_object_entity_name: ServerObjectEntityName,
    top_level_field_or_pointer: SelectionType<
        ParentObjectEntityNameAndSelectableName,
        ParentObjectEntityNameAndSelectableName,
    >,
) -> DiagnosticVecResult<WithSpan<SelectionSet<ScalarSelectableId, ObjectSelectableId>>> {
    let selections =
        get_all_errors_or_all_ok(selection_set.item.selections.into_iter().map(|selection| {
            get_validated_selection(
                db,
                selection,
                parent_object_entity_name,
                top_level_field_or_pointer,
            )
        }))?;

    SelectionSet { selections }.with_generated_span().wrap_ok()
}

fn get_validated_selection<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    with_span: WithSpan<UnvalidatedSelection>,
    parent_object_entity_name: ServerObjectEntityName,
    top_level_field_or_pointer: SelectionType<
        ParentObjectEntityNameAndSelectableName,
        ParentObjectEntityNameAndSelectableName,
    >,
) -> DiagnosticVecResult<WithSpan<ValidatedSelection>> {
    with_span.and_then(|selection| match selection {
        SelectionType::Scalar(scalar_selection) => get_validated_scalar_selection(
            db,
            parent_object_entity_name,
            top_level_field_or_pointer,
            scalar_selection,
        )?
        .scalar_selected()
        .wrap_ok(),
        SelectionType::Object(object_selection) => get_validated_object_selection(
            db,
            parent_object_entity_name,
            top_level_field_or_pointer,
            object_selection,
        )?
        .object_selected()
        .wrap_ok(),
    })
}

fn get_validated_scalar_selection<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    parent_object_entity_name: ServerObjectEntityName,
    top_level_field_or_pointer: SelectionType<
        ParentObjectEntityNameAndSelectableName,
        ParentObjectEntityNameAndSelectableName,
    >,
    scalar_selection: UnvalidatedScalarFieldSelection,
) -> DiagnosticResult<ValidatedScalarSelection> {
    let type_and_field = match top_level_field_or_pointer {
        SelectionType::Scalar(s) => s,
        SelectionType::Object(o) => o,
    };

    let selectable = selectable_named(
        db,
        parent_object_entity_name,
        scalar_selection.name.item.into(),
    );

    let location = selectable.clone_err()?.as_ref().ok_or_else(|| {
        selection_does_not_exist_diagnostic(
            top_level_field_or_pointer.client_type(),
            type_and_field.parent_object_entity_name,
            type_and_field.selectable_name,
            parent_object_entity_name,
            scalar_selection.name.item.into(),
            scalar_selection.name.location,
        )
    })?;

    let associated_data = match location {
        DefinitionLocation::Server(server_selectable_id) => {
            // TODO encode this in types
            if matches!(
                scalar_selection.scalar_selection_directive_set,
                ScalarSelectionDirectiveSet::Loadable(_)
            ) {
                let scalar_name = scalar_selection.name.item;
                return Diagnostic::new(
                    format!(
                        "`{scalar_name}` is a server field, and cannot be selected with `@loadable`"
                    ),
                    scalar_selection.name.location.wrap_some(),
                )
                .wrap_err();
            }

            let server_scalar_selectable = server_selectable_id
                .as_ref()
                .as_scalar_result()
                .as_ref()
                .map_err(|_| {
                    let client_type = top_level_field_or_pointer.client_type();
                    selection_wrong_selection_type_diagnostic(
                        client_type,
                        type_and_field.parent_object_entity_name,
                        type_and_field.selectable_name,
                        parent_object_entity_name,
                        scalar_selection.name.item.into(),
                        "an object",
                        "a scalar",
                        scalar_selection.name.location,
                    )
                })?
                .lookup(db);

            (
                server_scalar_selectable.parent_object_entity_name,
                server_scalar_selectable.name.item,
            )
                .server_defined()
        }
        DefinitionLocation::Client(client_type) => {
            let client_scalar_selectable =
                *client_type.as_ref().as_scalar().as_ref().ok_or_else(|| {
                    selection_wrong_selection_type_diagnostic(
                        top_level_field_or_pointer.client_type(),
                        type_and_field.parent_object_entity_name,
                        type_and_field.selectable_name,
                        parent_object_entity_name,
                        scalar_selection.name.item.into(),
                        "an object",
                        "a scalar",
                        scalar_selection.name.location,
                    )
                })?;
            (
                client_scalar_selectable.parent_object_entity_name,
                client_scalar_selectable.name.item,
            )
                .client_defined()
        }
    };

    Ok(ScalarSelection {
        name: scalar_selection.name,
        reader_alias: scalar_selection.reader_alias,
        associated_data,
        scalar_selection_directive_set: scalar_selection.scalar_selection_directive_set,
        arguments: scalar_selection.arguments,
    })
}

fn get_validated_object_selection<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    parent_object_entity_name: ServerObjectEntityName,
    top_level_field_or_pointer: SelectionType<
        ParentObjectEntityNameAndSelectableName,
        ParentObjectEntityNameAndSelectableName,
    >,
    object_selection: ObjectSelection<(), ()>,
) -> DiagnosticVecResult<ValidatedObjectSelection> {
    let type_and_field = match top_level_field_or_pointer {
        SelectionType::Scalar(s) => s,
        SelectionType::Object(o) => o,
    };

    // TODO this can be vastly simplified... it looks like we're looking up the same object
    // multiple times :) and the result we're returning might be in the parameters anyway.

    let selectable = selectable_named(
        db,
        parent_object_entity_name,
        object_selection.name.item.into(),
    );

    let selectable = selectable.clone_err()?.as_ref().ok_or_else(|| {
        vec![selection_does_not_exist_diagnostic(
            top_level_field_or_pointer.client_type(),
            type_and_field.parent_object_entity_name,
            type_and_field.selectable_name,
            parent_object_entity_name,
            object_selection.name.item.into(),
            object_selection.name.location,
        )]
    })?;

    let (associated_data, new_parent_object_entity_name) = match selectable {
        DefinitionLocation::Server(server_selectable) => {
            let server_object_selectable = server_selectable
                .as_ref()
                .as_object_result()
                .as_ref()
                .map_err(|server_scalar_selectable| {
                    let server_scalar_selectable = server_scalar_selectable.lookup(db);

                    vec![selection_wrong_selection_type_diagnostic(
                        top_level_field_or_pointer.client_type(),
                        type_and_field.parent_object_entity_name,
                        type_and_field.selectable_name,
                        parent_object_entity_name,
                        object_selection.name.item.into(),
                        "a scalar",
                        "an object",
                        server_scalar_selectable.name.location,
                    )]
                })?
                .lookup(db);

            (
                (
                    server_object_selectable.parent_object_entity_name,
                    server_object_selectable.name.item,
                )
                    .server_defined(),
                *server_object_selectable.target_object_entity.inner(),
            )
        }
        DefinitionLocation::Client(client_type) => {
            let client_object_selectable = *client_type
                .as_ref()
                .as_object_result()
                .as_ref()
                .map_err(|e| {
                    vec![selection_wrong_selection_type_diagnostic(
                        top_level_field_or_pointer.client_type(),
                        type_and_field.parent_object_entity_name,
                        type_and_field.selectable_name,
                        parent_object_entity_name,
                        object_selection.name.item.into(),
                        "a scalar",
                        "an object",
                        e.name.location,
                    )]
                })?;

            (
                (
                    client_object_selectable.parent_object_entity_name,
                    client_object_selectable.name.item,
                )
                    .client_defined(),
                *client_object_selectable.target_object_entity_name.inner(),
            )
        }
    };

    Ok(ObjectSelection {
        name: object_selection.name,
        reader_alias: object_selection.reader_alias,
        object_selection_directive_set: object_selection.object_selection_directive_set,
        associated_data,
        arguments: object_selection.arguments,
        selection_set: get_validated_selection_set(
            db,
            object_selection.selection_set,
            new_parent_object_entity_name,
            top_level_field_or_pointer,
        )?,
    })
}

pub fn get_validated_refetch_strategy<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    unvalidated_refetch_strategy: RefetchStrategy<(), ()>,
    parent_object_entity_name: ServerObjectEntityName,
    top_level_field_or_pointer: SelectionType<
        ParentObjectEntityNameAndSelectableName,
        ParentObjectEntityNameAndSelectableName,
    >,
) -> DiagnosticVecResult<RefetchStrategy<ScalarSelectableId, ObjectSelectableId>> {
    match unvalidated_refetch_strategy {
        RefetchStrategy::UseRefetchField(use_refetch_field_strategy) => Ok(
            RefetchStrategy::UseRefetchField(UseRefetchFieldRefetchStrategy {
                refetch_selection_set: get_validated_selection_set(
                    db,
                    use_refetch_field_strategy.refetch_selection_set,
                    parent_object_entity_name,
                    top_level_field_or_pointer,
                )?,
                root_fetchable_type_name: use_refetch_field_strategy.root_fetchable_type_name,
                generate_refetch_query: use_refetch_field_strategy.generate_refetch_query,
            }),
        ),
        RefetchStrategy::RefetchFromRoot => Ok(RefetchStrategy::RefetchFromRoot),
    }
}

pub fn get_all_errors_or_all_ok<T, E>(
    items: impl Iterator<Item = Result<T, Vec<E>>>,
) -> Result<Vec<T>, Vec<E>> {
    let mut oks = vec![];
    let mut errors = vec![];

    for item in items {
        match item {
            Ok(ok) => oks.push(ok),
            Err(e) => errors.extend(e),
        }
    }

    if errors.is_empty() {
        Ok(oks)
    } else {
        Err(errors)
    }
}

fn selection_does_not_exist_diagnostic(
    client_type: &str,
    declaration_parent_object_entity_name: ServerObjectEntityName,
    declaration_selectable_name: SelectableName,
    selectable_parent_object_entity_name: ServerObjectEntityName,
    selectable_name: SelectableName,
    location: Location,
) -> Diagnostic {
    Diagnostic::new(
        format!(
            "In the client {client_type} `{declaration_parent_object_entity_name}.{declaration_selectable_name}`, \
            the field `{selectable_parent_object_entity_name}.{selectable_name}` is selected, but that \
            field does not exist on `{selectable_parent_object_entity_name}`"
        ),
        location.wrap_some(),
    )
}

#[expect(clippy::too_many_arguments)]
fn selection_wrong_selection_type_diagnostic(
    client_type: &str,
    declaration_entity_name: ServerObjectEntityName,
    declaration_selectable_name: SelectableName,
    selectable_entity_name: ServerObjectEntityName,
    selectable_name: SelectableName,
    selected_as: &str,
    proper_way_to_select: &str,
    location: Location,
) -> Diagnostic {
    Diagnostic::new(
        format!(
            "In the client {client_type} \
            `{declaration_entity_name}.{declaration_selectable_name}`, \
            the field `{selectable_entity_name}.{selectable_name}` \
            is selected as {selected_as}. Instead, that field should be selected \
            as {proper_way_to_select}"
        ),
        location.wrap_some(),
    )
}
