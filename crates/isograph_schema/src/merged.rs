use std::collections::{
    hash_map::{Entry, OccupiedEntry, VacantEntry},
    HashMap,
};

use common_lang_types::{
    DefinedField, LinkedFieldAlias, LinkedFieldName, ScalarFieldAlias, ScalarFieldName,
    SelectableFieldName, ServerFieldNormalizationKey, Span, WithSpan,
};
use intern::{string_key::Intern, Lookup};
use isograph_lang_types::{
    LinkedFieldSelection, ObjectId, ScalarFieldSelection, Selection, SelectionFieldArgument,
    ServerFieldId, ServerFieldSelection,
};

use crate::{
    SchemaObject, ValidatedEncounteredDefinedField, ValidatedScalarDefinedField, ValidatedSchema,
    ValidatedSchemaIdField, ValidatedSchemaObject, ValidatedSelection,
};

type MergedSelectionMap = HashMap<NormalizationKey, WithSpan<MergedServerFieldSelection>>;

// TODO add id and typename variants, impl Ord, and get rid of the NormalizationKey enum
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug)]
pub enum MergedServerFieldSelection {
    ScalarField(MergedScalarFieldSelection),
    LinkedField(MergedLinkedFieldSelection),
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug)]
pub struct MergedScalarFieldSelection {
    pub name: WithSpan<ScalarFieldName>,
    pub normalization_alias: Option<WithSpan<ScalarFieldAlias>>,
    // TODO rename, since this isn't generic
    pub associated_data: ServerFieldId,
    pub arguments: Vec<WithSpan<SelectionFieldArgument>>,
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug)]
pub struct MergedLinkedFieldSelection {
    pub name: WithSpan<LinkedFieldName>,
    pub normalization_alias: Option<WithSpan<LinkedFieldAlias>>,
    // TODO rename, since this isn't generic
    pub associated_data: ObjectId,
    pub selection_set: Vec<WithSpan<MergedServerFieldSelection>>,
    pub arguments: Vec<WithSpan<SelectionFieldArgument>>,
}

pub struct MergedSelectionSet(Vec<WithSpan<MergedServerFieldSelection>>);

impl std::ops::Deref for MergedSelectionSet {
    type Target = Vec<WithSpan<MergedServerFieldSelection>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl MergedSelectionSet {
    fn new(
        mut unsorted_vec: Vec<(NormalizationKey, WithSpan<MergedServerFieldSelection>)>,
    ) -> Self {
        unsorted_vec.sort_by(|(k1, _), (k2, _)| k1.cmp(k2));
        MergedSelectionSet(unsorted_vec.into_iter().map(|(_, value)| value).collect())
    }
}

impl Into<Vec<WithSpan<MergedServerFieldSelection>>> for MergedSelectionSet {
    fn into(self) -> Vec<WithSpan<MergedServerFieldSelection>> {
        self.0
    }
}

#[derive(Debug, Eq, PartialEq, Clone, Copy, PartialOrd, Ord, Hash)]
enum NormalizationKey {
    // __typename,
    Id,
    ServerField(ServerFieldNormalizationKey),
}

/// A merged selection set is an input for generating:
/// - query texts
/// - normalization artifacts (TODO)
/// - raw response types (TODO)
///
/// TODO: SelectionSetAndUnwraps should be generic enough to handle this
pub fn create_merged_selection_set(
    schema: &ValidatedSchema,
    parent_type: &SchemaObject<ValidatedEncounteredDefinedField>,
    selection_set: &Vec<WithSpan<ValidatedSelection>>,
) -> MergedSelectionSet {
    let mut merged_selection_set = HashMap::new();

    merge_selections_into_set(
        schema,
        &mut merged_selection_set,
        parent_type,
        selection_set,
    );

    add_typename_and_id_fields(schema, &mut merged_selection_set, parent_type);

    MergedSelectionSet::new(merged_selection_set.into_iter().collect())
}

fn merge_selections_into_set(
    schema: &ValidatedSchema,
    merged_selection_set: &mut MergedSelectionMap,
    parent_type: &SchemaObject<ValidatedEncounteredDefinedField>,
    validated_selections: &Vec<WithSpan<ValidatedSelection>>,
) {
    for validated_selection in validated_selections.iter().filter(filter_id_fields) {
        let span = validated_selection.span;
        match &validated_selection.item {
            Selection::ServerField(validated_server_field) => match validated_server_field {
                ServerFieldSelection::ScalarField(scalar_field) => {
                    match &scalar_field.associated_data {
                        DefinedField::ServerField(server_field_id) => merge_scalar_server_field(
                            scalar_field,
                            merged_selection_set,
                            server_field_id,
                            span,
                        ),
                        DefinedField::ResolverField(_) => merge_scalar_resolver_field(
                            scalar_field,
                            parent_type,
                            schema,
                            merged_selection_set,
                        ),
                    };
                }
                ServerFieldSelection::LinkedField(new_linked_field) => {
                    let normalization_key = NormalizationKey::ServerField(
                        HACK_combine_name_and_variables_into_normalization_alias(
                            new_linked_field.name.item.into(),
                            &new_linked_field.arguments,
                        ),
                    );
                    match merged_selection_set.entry(normalization_key) {
                        Entry::Occupied(occupied) => merge_linked_field_into_occupied_entry(
                            occupied,
                            new_linked_field,
                            schema,
                        ),
                        Entry::Vacant(vacant_entry) => merge_linked_field_into_vacant_entry(
                            vacant_entry,
                            new_linked_field,
                            schema,
                            span,
                        ),
                    };
                }
            },
        }
    }
}

fn filter_id_fields(field: &&WithSpan<Selection<ValidatedScalarDefinedField, ObjectId>>) -> bool {
    // filter out id fields, and eventually other always-selected fields like __typename
    match &field.item {
        Selection::ServerField(server_field) => match server_field {
            ServerFieldSelection::ScalarField(scalar_field) => {
                // -------- HACK --------
                // Here, we check whether the field is named "id", but we should really
                // know whether it is an id field in some other way. There can be non-id fields
                // named id and id fields not named "id".
                scalar_field.name.item != "id".intern().into()
                // ------ END HACK ------
            }
            ServerFieldSelection::LinkedField(_) => true,
        },
    }
}

fn merge_linked_field_into_vacant_entry(
    vacant_entry: VacantEntry<'_, NormalizationKey, WithSpan<MergedServerFieldSelection>>,
    new_linked_field: &LinkedFieldSelection<ValidatedScalarDefinedField, ObjectId>,
    schema: &ValidatedSchema,
    span: Span,
) {
    vacant_entry.insert(WithSpan::new(
        MergedServerFieldSelection::LinkedField(MergedLinkedFieldSelection {
            name: new_linked_field.name,
            associated_data: new_linked_field.associated_data,
            selection_set: {
                let type_id = new_linked_field.associated_data;
                let linked_field_parent_type = schema.schema_data.object(type_id);
                create_merged_selection_set(
                    schema,
                    linked_field_parent_type,
                    &new_linked_field.selection_set,
                )
                .into()
            },
            arguments: new_linked_field.arguments.clone(),
            normalization_alias: new_linked_field.normalization_alias,
        }),
        span,
    ));
}

fn merge_linked_field_into_occupied_entry(
    mut occupied: OccupiedEntry<'_, NormalizationKey, WithSpan<MergedServerFieldSelection>>,
    new_linked_field: &LinkedFieldSelection<ValidatedScalarDefinedField, ObjectId>,
    schema: &ValidatedSchema,
) {
    let existing_selection = occupied.get_mut();
    match &mut existing_selection.item {
        MergedServerFieldSelection::ScalarField(_) => {
            panic!("expected linked, probably a bug in Isograph")
        }
        MergedServerFieldSelection::LinkedField(existing_linked_field) => {
            let type_id = new_linked_field.associated_data;
            let linked_field_parent_type = schema.schema_data.object(type_id);
            HACK__merge_linked_fields(
                schema,
                &mut existing_linked_field.selection_set,
                &new_linked_field.selection_set,
                linked_field_parent_type,
            );
        }
    }
}

fn merge_scalar_resolver_field(
    scalar_field: &ScalarFieldSelection<ValidatedScalarDefinedField>,
    parent_type: &SchemaObject<ValidatedEncounteredDefinedField>,
    schema: &ValidatedSchema,
    merged_selection_set: &mut MergedSelectionMap,
) {
    let resolver_field_name = scalar_field.name.item;
    let parent_field_id = parent_type
        .resolvers
        .iter()
        .find(|parent_field_id| {
            let field = schema.resolver(**parent_field_id);
            field.name == resolver_field_name.into()
        })
        .expect("expect field to exist");
    let resolver_field = schema.resolver(*parent_field_id);
    if let Some((ref selection_set, _)) = resolver_field.selection_set_and_unwraps {
        merge_selections_into_set(schema, merged_selection_set, parent_type, selection_set)
    }
}

fn merge_scalar_server_field(
    scalar_field: &ScalarFieldSelection<ValidatedScalarDefinedField>,
    merged_selection_set: &mut MergedSelectionMap,
    server_field_id: &ServerFieldId,
    span: Span,
) {
    let normalization_key =
        NormalizationKey::ServerField(HACK_combine_name_and_variables_into_normalization_alias(
            scalar_field.name.item.into(),
            &scalar_field.arguments,
        ));
    match merged_selection_set.entry(normalization_key) {
        Entry::Occupied(_) => {
            // TODO: do we need to check for merge conflicts on scalars? Not while
            // we are auto-aliasing.
        }
        Entry::Vacant(vacant_entry) => {
            vacant_entry.insert(WithSpan::new(
                MergedServerFieldSelection::ScalarField(MergedScalarFieldSelection {
                    name: scalar_field.name,
                    associated_data: *server_field_id,
                    arguments: scalar_field.arguments.clone(),
                    normalization_alias: scalar_field.normalization_alias,
                }),
                span,
            ));
        }
    }
}

/// In order to avoid requiring a normalization AST, we write the variables
/// used in the alias. Once we have a normalization AST, we can remove this.
#[allow(non_snake_case)]
fn HACK_combine_name_and_variables_into_normalization_alias(
    name: SelectableFieldName,
    arguments: &[WithSpan<SelectionFieldArgument>],
) -> ServerFieldNormalizationKey {
    if arguments.is_empty() {
        name.into()
    } else {
        let mut alias_str = name.to_string();

        for argument in arguments {
            alias_str.push_str(&format!(
                "__{}_{}",
                argument.item.name.item,
                &argument.item.value.item.to_string()[1..]
            ));
        }
        alias_str.intern().into()
    }
}

/// LinkedFieldSelection contains a selection set that is a Vec<...>, but we
/// really want it to be a HashMap<...>. However, we can't really do that because
/// LinkdFieldSelection has both field: TLinkedField and
/// selection_set: Vec<..., TLinkedField, ...>. If we make LinkedFieldSelection
/// generic over both TLinkedField and TSelectionSet, then we get some recursive
/// definition error.
///
/// TODO figure out a way around that!
///
/// In this function, we convert the Vec to a HashMap, do the merging, then
/// convert back. Blah!
#[allow(non_snake_case)]
fn HACK__merge_linked_fields(
    schema: &ValidatedSchema,
    existing_selection_set: &mut Vec<WithSpan<MergedServerFieldSelection>>,
    new_selection_set: &Vec<WithSpan<ValidatedSelection>>,
    linked_field_parent_type: &SchemaObject<ValidatedEncounteredDefinedField>,
) {
    let mut merged_selection_set = HashMap::new();
    for item in existing_selection_set.iter() {
        let span = item.span;
        match &item.item {
            MergedServerFieldSelection::ScalarField(scalar_field) => {
                // N.B. if you have a field named "id" which is a linked field, this will probably
                // work incorrectly!
                let normalization_key = NormalizationKey::ServerField(
                    HACK_combine_name_and_variables_into_normalization_alias(
                        scalar_field.name.item.into(),
                        &scalar_field.arguments,
                    ),
                );

                merged_selection_set.insert(
                    normalization_key,
                    WithSpan::new(
                        MergedServerFieldSelection::ScalarField(scalar_field.clone()),
                        span,
                    ),
                )
            }
            MergedServerFieldSelection::LinkedField(linked_field) => {
                let normalization_key = NormalizationKey::ServerField(
                    HACK_combine_name_and_variables_into_normalization_alias(
                        linked_field.name.item.into(),
                        &linked_field.arguments,
                    ),
                );
                merged_selection_set.insert(
                    normalization_key,
                    WithSpan::new(
                        MergedServerFieldSelection::LinkedField(linked_field.clone()),
                        span,
                    ),
                )
            }
        };
    }

    merge_selections_into_set(
        schema,
        &mut merged_selection_set,
        linked_field_parent_type,
        new_selection_set,
    );

    let mut merged_fields: Vec<_> = merged_selection_set
        .into_iter()
        .map(|(_key, value)| value)
        .collect();
    merged_fields.sort();

    *existing_selection_set = merged_fields;
}

fn add_typename_and_id_fields(
    schema: &ValidatedSchema,
    merged_selection_set: &mut MergedSelectionMap,
    parent_type: &ValidatedSchemaObject,
) {
    // TODO add __typename field or whatnot

    let id_field: Option<ValidatedSchemaIdField> = parent_type
        .id_field
        .map(|id_field_id| schema.id_field(id_field_id));

    // If the type has an id field, we must select it.
    if let Some(id_field) = id_field {
        match merged_selection_set.entry(NormalizationKey::Id) {
            Entry::Occupied(_) => {
                // TODO: do we need to check for merge conflicts on scalars? Not while
                // we are auto-aliasing.
            }
            Entry::Vacant(vacant_entry) => {
                vacant_entry.insert(WithSpan::new(
                    MergedServerFieldSelection::ScalarField(MergedScalarFieldSelection {
                        // major HACK alert
                        name: WithSpan::new(
                            id_field.name.lookup().intern().into(),
                            Span::new(0, 0),
                        ),
                        associated_data: id_field.id.into(),
                        arguments: vec![],
                        // This indicates that there should be a separate MergedServerFieldSelection variant
                        normalization_alias: None,
                    }),
                    Span::new(0, 0),
                ));
            }
        }
    }
}
