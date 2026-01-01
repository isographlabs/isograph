use std::{fmt::Debug, hash::Hash};

use common_lang_types::EntityName;

use crate::{CompilationProfile, IsographDatabase};

pub trait TargetPlatform:
    Debug + Clone + Copy + Eq + PartialEq + Ord + PartialOrd + Hash + Default + Sized + 'static
{
    type EntityAssociatedData: Debug + PartialEq + Eq + Clone + Hash;

    fn format_server_field_scalar_type<
        TCompilationProfile: CompilationProfile<TargetPlatform = Self>,
    >(
        db: &IsographDatabase<TCompilationProfile>,
        entity_name: EntityName,
        indentation_level: u8,
    ) -> String;
}
