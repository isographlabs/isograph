use common_lang_types::{
    DescriptionValue, Diagnostic, EntityName, SelectableName, WithLocationPostfix,
};
use intern::string_key::Intern;
use isograph_lang_types::{
    DefinitionLocationPostfix, Description, EntityNameWrapper, SelectionType, SelectionTypePostfix,
    TypeAnnotationDeclaration,
};
use pico_macros::memo;
use prelude::Postfix;

use crate::{
    ClientFieldVariant, CompilationProfile, FlattenedDataModelEntity, FlattenedDataModelSelectable,
    IsInlineFragment, IsoLiteralExportInfo, IsographDatabase, UserWrittenClientTypeInfo,
    client_selectable_declaration_map_from_iso_literals,
};

#[memo]
pub fn flattened_client_schema<TCompilationProfile: CompilationProfile>(
    _db: &IsographDatabase<TCompilationProfile>,
) -> (
    Vec<FlattenedDataModelEntity<TCompilationProfile>>,
    Vec<FlattenedDataModelSelectable<TCompilationProfile>>,
    // Non-fatal diagnostics
    Vec<Diagnostic>,
) {
    let entities = vec![];
    let selectables = vec![];
    let diagnostics = vec![];

    (entities, selectables, diagnostics)
}

pub fn define_client_selectables_and_entities<TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
    entities: &mut Vec<FlattenedDataModelEntity<TCompilationProfile>>,
    selectables: &mut Vec<FlattenedDataModelSelectable<TCompilationProfile>>,
    _non_fatal_diagnostics: &mut Vec<Diagnostic>,
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

                // TODO handle duplicates
                entities.push(FlattenedDataModelEntity {
                    name: target_entity_name.with_no_location(),
                    description: format!(
                        "The anonymous entity for the `{}.{}` client field",
                        scalar_declaration.parent_type.item,
                        scalar_declaration.client_field_name.item
                    )
                    .intern()
                    .to::<DescriptionValue>()
                    .to::<Description>()
                    .with_no_location()
                    .wrap_some(),
                    selectables: Default::default(),
                    associated_data: ().client_defined(),
                    selection_info: ().scalar_selected(),
                });

                // TODO handle duplicates
                selectables.push(FlattenedDataModelSelectable {
                    name: scalar_declaration
                        .client_field_name
                        // .map_location(Some)
                        .drop_location()
                        .map(|x| x.0),
                    parent_entity_name: scalar_declaration
                        .parent_type
                        // .map_location(Some)
                        .drop_location()
                        .map(|x| x.0),
                    description: scalar_declaration.description.map(|d| d.drop_location()),
                    // .map_location(Some)),
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
                    .with_no_location(),
                    associated_data: ClientFieldVariant::UserWritten(UserWrittenClientTypeInfo {
                        info: IsoLiteralExportInfo {
                            const_export_name: scalar_declaration.const_export_name,
                            file_path: scalar_declaration.definition_path,
                        },
                        directive_set: scalar_declaration.directive_set.clone(),
                    })
                    .client_defined(),
                    is_inline_fragment: IsInlineFragment(false),
                });
            }
            SelectionType::Object(object_declaration) => {
                let object_declaration = object_declaration.lookup(db);

                // TODO handle duplicates
                selectables.push(FlattenedDataModelSelectable {
                    name: object_declaration
                        .client_pointer_name
                        // .map_location(Some)
                        .drop_location()
                        .map(|x| x.0.to::<SelectableName>()),
                    parent_entity_name: object_declaration
                        .parent_type
                        // .map_location(Some)
                        .drop_location()
                        .map(|x| x.0),
                    description: object_declaration.description.map(
                        |d| d.drop_location(), // .map_location(Some)
                    ),
                    arguments: object_declaration
                        .variable_definitions
                        .clone()
                        .into_iter()
                        .map(|v| v.item)
                        .collect(),
                    target_entity: object_declaration
                        .target_type
                        .clone()
                        // .map_location(Some)
                        .drop_location()
                        .map(Ok),
                    associated_data: ClientFieldVariant::UserWritten(UserWrittenClientTypeInfo {
                        info: IsoLiteralExportInfo {
                            const_export_name: object_declaration.const_export_name,
                            file_path: object_declaration.definition_path,
                        },
                        directive_set: object_declaration.directives.clone(),
                    })
                    .client_defined(),
                    is_inline_fragment: IsInlineFragment(false),
                });
            }
        }
    }
}
