use common_lang_types::SelectableName;

use crate::{
    CompilationProfile, DataModelEntity, DataModelSelectable, FlattenedDataModelEntity,
    FlattenedDataModelSelectable, MapWithNonfatalDiagnostics, NestedDataModelEntity,
    NestedDataModelSelectable,
};

impl<TCompilationProfile: CompilationProfile> NestedDataModelEntity<TCompilationProfile> {
    pub fn flatten(
        self,
    ) -> (
        FlattenedDataModelEntity<TCompilationProfile>,
        // TODO use traits and associated types...
        MapWithNonfatalDiagnostics<
            SelectableName,
            (FlattenedDataModelSelectable<TCompilationProfile>, ()),
        >,
    ) {
        let selectables = self
            .selectables
            .item
            .into_iter()
            .map(|(key, value)| (key, value.flatten()))
            .collect();

        (
            DataModelEntity {
                name: self.name.drop_location(),
                description: self.description.map(|x| x.drop_location()),
                selectables: (),
                network_protocol_associated_data: self.network_protocol_associated_data,
                target_platform_associated_data: self.target_platform_associated_data,
                selection_info: self.selection_info,
            },
            MapWithNonfatalDiagnostics::new(selectables, self.selectables.non_fatal_diagnostics),
        )
    }
}

impl<TCompilationProfile: CompilationProfile> NestedDataModelSelectable<TCompilationProfile> {
    pub fn flatten(self) -> (FlattenedDataModelSelectable<TCompilationProfile>, ()) {
        (
            DataModelSelectable {
                name: self.name.drop_location(),
                parent_entity_name: self.parent_entity_name.drop_location(),
                description: self.description.map(|x| x.drop_location()),
                arguments: self.arguments,
                target_entity: self.target_entity.drop_location(),
                network_protocol_associated_data: self.network_protocol_associated_data,
                target_platform_associated_data: self.target_platform_associated_data,
                is_inline_fragment: self.is_inline_fragment,
            },
            (),
        )
    }
}
