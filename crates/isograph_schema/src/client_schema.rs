use common_lang_types::{
    DescriptionValue, EntityName, Location, SelectableName, WithLocationPostfix,
};
use intern::string_key::Intern;
use isograph_lang_types::{
    DefinitionLocationPostfix, Description, EntityNameWrapper, SelectionType, SelectionTypePostfix,
    TypeAnnotationDeclaration,
};
use pico_macros::memo;
use prelude::Postfix;

use crate::{
    CompilationProfile, FlattenedDataModelSchema, IsInlineFragment, IsographDatabase,
    NestedDataModelEntity, NestedDataModelSchema, NestedDataModelSelectable,
    client_selectable_declaration_map_from_iso_literals, entity_not_defined_diagnostic,
    flatten::Flatten, insert_entity_into_schema_or_emit_multiple_definitions_diagnostic,
    insert_selectable_into_schema_or_emit_multiple_definitions_diagnostic,
};

#[memo]
pub fn flattened_client_schema<TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
) -> FlattenedDataModelSchema<TCompilationProfile> {
    nested_client_schema(db)
        .item
        .iter()
        .map(|(key, value)| {
            (
                key.dereference(),
                value.clone().drop_location().map(|x| x.flatten()),
            )
        })
        .collect()
}

// TODO return a schema that is flattened, but contains locations
#[memo]
pub fn nested_client_schema<TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
) -> NestedDataModelSchema<TCompilationProfile> {
    let mut schema = Default::default();

    insert_client_selectables_into_schema(db, &mut schema);

    schema
}

pub fn insert_client_selectables_into_schema<TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
    schema: &mut NestedDataModelSchema<TCompilationProfile>,
) {
    for declaration in client_selectable_declaration_map_from_iso_literals(db)
        .item
        .reference()
        .values()
    {
        match declaration.item {
            SelectionType::Scalar(scalar_declaration) => {
                let scalar_declaration = scalar_declaration.lookup(db);
                let target_entity_name = format!(
                    "{}__{}__targetEntity",
                    scalar_declaration.parent_type.item, scalar_declaration.client_field_name.item
                )
                .intern()
                .to::<EntityName>();

                insert_entity_into_schema_or_emit_multiple_definitions_diagnostic(
                    schema,
                    NestedDataModelEntity {
                        name: target_entity_name.with_missing_location(),
                        description: format!(
                            "The anonymous entity for the {}.{} client field",
                            scalar_declaration.parent_type.item,
                            scalar_declaration.client_field_name.item
                        )
                        .intern()
                        .to::<DescriptionValue>()
                        .to::<Description>()
                        .with_missing_location()
                        .wrap_some(),
                        selectables: Default::default(),
                        associated_data: ().client_defined(),
                        selection_info: ().scalar_selected(),
                    }
                    .with_some_location(declaration.location),
                );

                let Some(parent_entity) =
                    schema.item.get_mut(&scalar_declaration.parent_type.item.0)
                else {
                    schema
                        .non_fatal_diagnostics
                        .push(entity_not_defined_diagnostic(
                            scalar_declaration.parent_type.item.0,
                            declaration.location.to::<Location>().wrap_some(),
                        ));
                    continue;
                };

                insert_selectable_into_schema_or_emit_multiple_definitions_diagnostic(
                    &mut parent_entity.item.selectables,
                    NestedDataModelSelectable {
                        name: scalar_declaration
                            .client_field_name
                            .map_location(Some)
                            .map(|x| x.0),
                        parent_entity_name: scalar_declaration
                            .parent_type
                            .map_location(Some)
                            .map(|x| x.0),
                        description: scalar_declaration.description.map(|d| d.map_location(Some)),
                        arguments: scalar_declaration
                            .variable_definitions
                            .clone()
                            .into_iter()
                            .map(|v| v.item)
                            .collect(),
                        target_entity: TypeAnnotationDeclaration::Scalar(
                            target_entity_name.to::<EntityNameWrapper>(),
                        )
                        .wrap_ok()
                        .with_missing_location(),
                        associated_data: ().client_defined(),
                        is_inline_fragment: IsInlineFragment(false),
                    }
                    .with_some_location(declaration.location),
                );
            }
            SelectionType::Object(object_declaration) => {
                let object_declaration = object_declaration.lookup(db);

                let Some(parent_entity) =
                    schema.item.get_mut(&object_declaration.parent_type.item.0)
                else {
                    schema
                        .non_fatal_diagnostics
                        .push(entity_not_defined_diagnostic(
                            object_declaration.parent_type.item.0,
                            declaration.location.to::<Location>().wrap_some(),
                        ));
                    continue;
                };

                insert_selectable_into_schema_or_emit_multiple_definitions_diagnostic(
                    &mut parent_entity.item.selectables,
                    NestedDataModelSelectable {
                        name: object_declaration
                            .client_pointer_name
                            .map_location(Some)
                            .map(|x| x.0.to::<SelectableName>()),
                        parent_entity_name: object_declaration
                            .parent_type
                            .map_location(Some)
                            .map(|x| x.0),
                        description: object_declaration.description.map(|d| d.map_location(Some)),
                        arguments: object_declaration
                            .variable_definitions
                            .clone()
                            .into_iter()
                            .map(|v| v.item)
                            .collect(),
                        target_entity: object_declaration
                            .target_type
                            .clone()
                            .map_location(Some)
                            .map(Ok),
                        associated_data: ().client_defined(),
                        is_inline_fragment: IsInlineFragment(false),
                    }
                    .with_some_location(declaration.location),
                );
            }
        }
    }
}
