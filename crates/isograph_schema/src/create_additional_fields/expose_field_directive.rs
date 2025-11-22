use common_lang_types::{
    ParentObjectEntityNameAndSelectableName, SelectableName, ServerObjectEntityName,
    ServerObjectSelectableName, ServerSelectableName, StringLiteralValue, WithLocation,
    WithSpanPostfix,
};
use intern::{Lookup, string_key::Intern};
use isograph_lang_types::{
    EmptyDirectiveSet, ScalarSelection, ScalarSelectionDirectiveSet, SelectionType,
    SelectionTypeContainingSelections, VariableDefinition,
};

use prelude::Postfix;
use serde::Deserialize;

use crate::{
    ClientFieldVariant, ClientScalarSelectable, ExposeFieldToInsert,
    ImperativelyLoadedFieldVariant, IsographDatabase, NetworkProtocol, RefetchStrategy,
    ServerEntityName, ServerObjectSelectableVariant, UnprocessedClientScalarSelectableSelectionSet,
    WrappedSelectionMapSelection, create_additional_fields::argument_map::remove_field_map_item,
    generate_refetch_field_strategy, get_object_selections_path,
    imperative_field_subfields_or_inline_fragments, server_object_entity_named,
    server_selectable_named,
};

use super::{
    argument_map::ArgumentMap,
    create_additional_fields_error::{
        CreateAdditionalFieldsError, FieldMapItem, ProcessTypeDefinitionResult,
        ProcessedFieldMapItem,
    },
};

// TODO move to graphql_network_protocol crate
#[derive(Deserialize, Eq, PartialEq, Debug, Hash, Clone)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ExposeFieldDirective {
    // TODO make this a ScalarSelectableName
    #[serde(default)]
    #[serde(rename = "as")]
    pub expose_as: Option<SelectableName>,
    #[serde(default)]
    pub field_map: Vec<FieldMapItem>,
    pub field: StringLiteralValue,
}

impl ExposeFieldDirective {
    pub fn new(
        expose_as: Option<SelectableName>,
        field_map: Vec<FieldMapItem>,
        field: StringLiteralValue,
    ) -> Self {
        Self {
            expose_as,
            field_map,
            field,
        }
    }
}

pub fn create_new_exposed_field<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    expose_field_to_insert: &ExposeFieldToInsert,
    parent_object_entity_name: ServerObjectEntityName,
) -> Result<
    (
        UnprocessedClientScalarSelectableSelectionSet,
        ClientScalarSelectable<TNetworkProtocol>,
    ),
    CreateAdditionalFieldsError<TNetworkProtocol>,
> {
    let ExposeFieldDirective {
        expose_as,
        field_map,
        field,
    } = &expose_field_to_insert.expose_field_directive;

    // HACK: we're essentially splitting the field arg by . and keeping the same
    // implementation as before. But really, there isn't much a distinction
    // between field and path, and we should clean this up.
    //
    // But, this is an expedient way to combine field and path.
    let mut path = field.lookup().split('.');
    let field = path.next().expect(
        "Expected iter to have at least one element. \
        This is indicative of a bug in Isograph.",
    );
    let primary_field_name_selection_parts = path.map(|x| x.intern().into()).collect::<Vec<_>>();

    let (parent_object_entity_name, mutation_subfield_name) =
        parse_mutation_subfield_id(db, field, parent_object_entity_name)?;

    let mutation_field =
        server_selectable_named(db, parent_object_entity_name, mutation_subfield_name.into())
            .as_ref()
            .map_err(|e| e.clone())?
            .as_ref()
            // TODO propagate this errors instead of panicking
            .expect(
                "Expected selectable to exist. \
                This is indicative of a bug in Isograph.",
            )
            .as_ref()
            .map_err(|e| e.clone())?
            .as_ref()
            .as_object()
            // TODO propagate this errors instead of panicking
            .expect(
                "Expected selectable to be an object selectable. \
                This is indicative of a bug in Isograph.",
            );

    let payload_object_type_annotation = &mutation_field.target_object_entity;
    let payload_object_entity_name = *payload_object_type_annotation.inner();

    let client_field_scalar_selection_name = expose_as.unwrap_or(mutation_field.name.item.into());
    // TODO what is going on here. Should mutation_field have a checked way of converting to LinkedField?
    let top_level_schema_field_name = mutation_field.name.item.unchecked_conversion();
    let mutation_field_arguments = mutation_field.arguments.clone();
    let description = expose_field_to_insert
        .description
        .or(mutation_field.description);

    let processed_field_map_items = skip_arguments_contained_in_field_map(
        db,
        mutation_field_arguments.clone(),
        payload_object_entity_name,
        expose_field_to_insert.parent_object_name,
        client_field_scalar_selection_name,
        // TODO don't clone
        field_map.clone(),
    )?;

    let top_level_schema_field_concrete_type =
        server_object_entity_named(db, payload_object_entity_name)
            .as_ref()
            .map_err(|e| e.clone())?
            .as_ref()
            .expect(
                "Expected entity to exist. \
            This is indicative of a bug in Isograph.",
            )
            .item
            .concrete_type;

    let (maybe_abstract_parent_object_entity_name, primary_field_concrete_type) =
        traverse_object_selections(
            db,
            payload_object_entity_name,
            &primary_field_name_selection_parts,
        )?;

    let fields = processed_field_map_items
        .iter()
        .map(|field_map_item| {
            let scalar_field_selection = ScalarSelection {
                name: WithLocation::new_generated(
                    // TODO make this no-op
                    // TODO split on . here; we should be able to have from: "best_friend.id" or whatnot.
                    field_map_item.0.from.unchecked_conversion(),
                ),
                reader_alias: None,
                associated_data: (),
                scalar_selection_directive_set: ScalarSelectionDirectiveSet::None(
                    EmptyDirectiveSet {},
                ),
                // TODO what about arguments? How would we handle them?
                arguments: vec![],
            };

            SelectionTypeContainingSelections::Scalar(scalar_field_selection).with_generated_span()
        })
        .collect::<Vec<_>>();

    let mutation_field_client_field_name =
        client_field_scalar_selection_name.unchecked_conversion();

    let top_level_schema_field_arguments = mutation_field_arguments
        .into_iter()
        .map(|x| x.item)
        .collect::<Vec<_>>();

    let mut parts_reversed = get_object_selections_path(
        db,
        payload_object_entity_name,
        primary_field_name_selection_parts.into_iter(),
    )?;
    parts_reversed.reverse();

    let mut subfields_or_inline_fragments = parts_reversed
        .iter()
        .map(|server_object_selectable| {
            // The server object selectable may represent a linked field or an inline fragment
            let x = match server_object_selectable.object_selectable_variant {
                ServerObjectSelectableVariant::LinkedField => {
                    WrappedSelectionMapSelection::LinkedField {
                        server_object_selectable_name: server_object_selectable.name.item,
                        arguments: vec![],
                        concrete_type: primary_field_concrete_type,
                    }
                }
                ServerObjectSelectableVariant::InlineFragment => {
                    WrappedSelectionMapSelection::InlineFragment(
                        server_object_entity_named(
                            db,
                            *server_object_selectable.target_object_entity.inner(),
                        )
                        .as_ref()
                        .map_err(|e| e.clone())?
                        .as_ref()
                        .expect(
                            "Expected entity to exist. \
                                This is indicative of a bug in Isograph.",
                        )
                        .item
                        .name,
                    )
                }
            };
            Ok::<_, CreateAdditionalFieldsError<TNetworkProtocol>>(x)
        })
        .collect::<Result<Vec<_>, _>>()?;

    subfields_or_inline_fragments.push(imperative_field_subfields_or_inline_fragments(
        top_level_schema_field_name,
        &top_level_schema_field_arguments,
        top_level_schema_field_concrete_type,
    ));

    let mutation_client_scalar_selectable = ClientScalarSelectable {
        description,
        name: WithLocation::new_generated(
            client_field_scalar_selection_name.unchecked_conversion(),
        ),
        variant: ClientFieldVariant::ImperativelyLoadedField(ImperativelyLoadedFieldVariant {
            top_level_schema_field_arguments,
            client_selection_name: client_field_scalar_selection_name.unchecked_conversion(),

            root_object_entity_name: parent_object_entity_name,
            subfields_or_inline_fragments: subfields_or_inline_fragments.clone(),
            field_map: field_map.clone(),
        }),
        variable_definitions: vec![],
        type_and_field: ParentObjectEntityNameAndSelectableName {
            parent_object_entity_name: maybe_abstract_parent_object_entity_name
                .unchecked_conversion(), // e.g. Pet
            selectable_name: client_field_scalar_selection_name, // set_pet_best_friend
        },
        parent_object_entity_name: maybe_abstract_parent_object_entity_name,
        network_protocol: std::marker::PhantomData,
    };

    (
        UnprocessedClientScalarSelectableSelectionSet {
            client_scalar_selectable_name: mutation_field_client_field_name,
            parent_object_entity_name: maybe_abstract_parent_object_entity_name,
            reader_selection_set: vec![],
            refetch_strategy: RefetchStrategy::UseRefetchField(generate_refetch_field_strategy(
                fields.to_vec(),
                // NOTE: this will probably panic if we're not exposing fields which are
                // originally on Mutation
                parent_object_entity_name,
                subfields_or_inline_fragments,
            ))
            .some(),
        },
        mutation_client_scalar_selectable,
    )
        .ok()
}

/// Here, we are turning "pet" (the field_arg) to the ServerFieldId
/// of that specific field
fn parse_mutation_subfield_id<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    field_arg: &str,
    mutation_object_entity_name: ServerObjectEntityName,
) -> ProcessTypeDefinitionResult<
    (ServerObjectEntityName, ServerObjectSelectableName),
    TNetworkProtocol,
> {
    let opt_field =
        server_selectable_named(db, mutation_object_entity_name, field_arg.intern().into())
            .as_ref()
            .map_err(|e| e.clone())?;

    match opt_field {
        Some(s) => {
            let server_object_selectable = s
                .as_ref()
                .map_err(|e| e.clone())?
                .as_ref()
                .as_object()
                .ok_or_else(|| CreateAdditionalFieldsError::InvalidField {
                    field_arg: field_arg.to_string(),
                })?;

            (
                server_object_selectable.parent_object_entity_name,
                server_object_selectable.name.item,
            )
                .ok()
        }
        None => CreateAdditionalFieldsError::InvalidField {
            field_arg: field_arg.to_string(),
        }
        .err(),
    }
}

fn skip_arguments_contained_in_field_map<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    arguments: Vec<WithLocation<VariableDefinition<ServerEntityName>>>,
    primary_type_name: ServerObjectEntityName,
    mutation_object_name: ServerObjectEntityName,
    mutation_field_name: SelectableName,
    field_map_items: Vec<FieldMapItem>,
) -> ProcessTypeDefinitionResult<Vec<ProcessedFieldMapItem>, TNetworkProtocol> {
    let mut processed_field_map_items = Vec::with_capacity(field_map_items.len());
    // TODO
    // We need to create entirely new arguments, which are the existing arguments minus
    // any paths that are in the field map.
    let mut argument_map = ArgumentMap::new(arguments);

    for field_map_item in field_map_items {
        processed_field_map_items.push(remove_field_map_item(
            db,
            &mut argument_map,
            field_map_item,
            primary_type_name,
            mutation_object_name,
            mutation_field_name,
        )?);
    }

    Ok(processed_field_map_items)
}

fn traverse_object_selections<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    root_object_name: ServerObjectEntityName,
    selections: &[ServerSelectableName],
) -> Result<
    (ServerObjectEntityName, Option<ServerObjectEntityName>),
    CreateAdditionalFieldsError<TNetworkProtocol>,
> {
    let mut current_entity_name = root_object_name;

    for selection_name in selections {
        let current_selectable = server_selectable_named(db, current_entity_name, *selection_name)
            .as_ref()
            .map_err(|e| e.clone())?;

        match current_selectable {
            Some(entity) => {
                let entity = entity.as_ref().map_err(|e| e.clone())?;
                match entity {
                    SelectionType::Scalar(_) => {
                        // TODO show a better error message
                        return CreateAdditionalFieldsError::InvalidField {
                            field_arg: selection_name.lookup().to_string(),
                        }
                        .err();
                    }
                    SelectionType::Object(object) => {
                        current_entity_name = *object.target_object_entity.inner();
                    }
                }
            }
            None => {
                return CreateAdditionalFieldsError::PrimaryDirectiveFieldNotFound {
                    primary_object_entity_name: current_entity_name,
                    field_name: selection_name.unchecked_conversion(),
                }
                .err();
            }
        };
    }

    let current_entity_concrete_type = server_object_entity_named(db, current_entity_name)
        .as_ref()
        .map_err(|e| e.clone())?
        .as_ref()
        .expect(
            "Expected entity to exist. \
            This is indicative of a bug in Isograph.",
        )
        .item
        .concrete_type;

    (current_entity_name, current_entity_concrete_type).ok()
}
