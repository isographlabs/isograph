use common_lang_types::{DescriptionValue, EntityName, WithLocationPostfix};
use intern::string_key::Intern;
use isograph_lang_types::{
    DefinitionLocationPostfix, Description, SelectionType, SelectionTypePostfix,
};
use pico_macros::memo;
use prelude::Postfix;

use crate::{
    CompilationProfile, FlattenedDataModelSchema, IsographDatabase, NestedDataModelEntity,
    NestedDataModelSchema, client_selectable_declaration_map_from_iso_literals, flatten::Flatten,
    insert_entity_into_schema_or_emit_multiple_definitions_diagnostic,
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

    create_target_entity_for_each_client_declaration(db, &mut schema);

    schema
}

pub fn create_target_entity_for_each_client_declaration<TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
    schema: &mut NestedDataModelSchema<TCompilationProfile>,
) {
    let declarations = client_selectable_declaration_map_from_iso_literals(db);

    for declaration in declarations.item.reference().values() {
        match declaration.item {
            SelectionType::Scalar(scalar_declaration) => {
                let scalar_declaration = scalar_declaration.lookup(db);
                insert_entity_into_schema_or_emit_multiple_definitions_diagnostic(
                    schema,
                    NestedDataModelEntity {
                        name: format!(
                            "{}__{}__targetEntity",
                            scalar_declaration.parent_type.item,
                            scalar_declaration.client_field_name.item
                        )
                        .intern()
                        .to::<EntityName>()
                        .with_missing_location(),
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
            }
            SelectionType::Object(_object_declaration) => {}
        }
    }
}
