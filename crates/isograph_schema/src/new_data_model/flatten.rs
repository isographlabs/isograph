use common_lang_types::SelectableName;

use crate::{
    CompilationProfile, DataModelEntity, DataModelSelectable, DataModelStage,
    FlattenedDataModelEntity, FlattenedDataModelSelectable, MapWithNonfatalDiagnostics,
    NestedDataModelEntity, NestedDataModelSelectable, NestedStage,
};

pub type BothFlattenedResults<TFlatten> = (
    <TFlatten as Flatten>::Output,
    <TFlatten as Flatten>::NestedOutput,
);

pub trait Flatten {
    type Output;
    type NestedOutput;

    fn flatten(self) -> BothFlattenedResults<Self>;
}

impl<TCompilationProfile: CompilationProfile> Flatten
    for NestedDataModelEntity<TCompilationProfile>
{
    type Output = FlattenedDataModelEntity<TCompilationProfile>;
    type NestedOutput = MapWithNonfatalDiagnostics<
        SelectableName,
        BothFlattenedResults<NestedDataModelSelectable<TCompilationProfile>>,
        <NestedStage as DataModelStage>::Error,
    >;
    fn flatten(self) -> (Self::Output, Self::NestedOutput) {
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
                associated_data: self.associated_data,
                selection_info: self.selection_info,
            },
            MapWithNonfatalDiagnostics::new(selectables, self.selectables.non_fatal_diagnostics),
        )
    }
}

impl<TCompilationProfile: CompilationProfile> Flatten
    for NestedDataModelSelectable<TCompilationProfile>
{
    type Output = FlattenedDataModelSelectable<TCompilationProfile>;
    type NestedOutput = ();

    fn flatten(self) -> BothFlattenedResults<Self> {
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
