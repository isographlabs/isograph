use std::{fmt::Debug, hash::Hash};

use common_lang_types::{EntityName, JavascriptName, SelectableName};

use crate::{CompilationProfile, IsographDatabase};

pub trait TargetPlatform:
    Debug + Clone + Copy + Eq + PartialEq + Ord + PartialOrd + Hash + Default + Sized + 'static
{
    type EntityAssociatedData: Debug + PartialEq + Eq + Clone + Hash + Ord + PartialOrd;
    type SelectableAssociatedData: Debug + PartialEq + Eq + Clone + Hash + Ord + PartialOrd;

    fn format_server_field_scalar_type<
        TCompilationProfile: CompilationProfile<TargetPlatform = Self>,
    >(
        db: &IsographDatabase<TCompilationProfile>,
        entity_name: EntityName,
        indentation_level: u8,
    ) -> String;

    fn get_inner_text_for_selectable<
        TCompilationProfile: CompilationProfile<TargetPlatform = Self>,
    >(
        db: &IsographDatabase<TCompilationProfile>,
        parent_object_entity_name: EntityName,
        selectable_name: SelectableName,
    ) -> JavascriptName;

    // TODO replace this with an entity with a JavascriptName, similar to how __typename
    // fields work
    fn generate_link_type<TCompilationProfile: CompilationProfile<TargetPlatform = Self>>(
        db: &IsographDatabase<TCompilationProfile>,
        server_object_entity_name: &EntityName,
    ) -> String;
}
