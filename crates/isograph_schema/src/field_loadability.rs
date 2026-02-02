//! TODO this is deprecated and should be removed

use isograph_lang_types::{LoadableDirectiveParameters, ScalarSelectionDirectiveSet};

use crate::{ClientFieldVariant, ImperativelyLoadedFieldVariant};

pub enum Loadability<'a> {
    LoadablySelectedField(&'a LoadableDirectiveParameters),
    ImperativelyLoadedField(&'a ImperativelyLoadedFieldVariant),
}

/// Why do we do this? Because how we handle a field is determined by both the
/// the field defition (e.g. exposed fields can only be fetched imperatively)
/// and the selection (i.e. we can also take non-imperative fields and make them
/// imperative.)
///
/// The eventual plan is to clean this model up. Instead, imperative fields will
/// need to be explicitly selected loadably. If they are not, they will be fetched
/// as an immediate follow-up request. Once we do this, there will always be one
/// source of truth for whether a field is fetched imperatively: the presence of the
/// @loadable directive.
pub fn categorize_field_loadability<'a>(
    variant: &'a ClientFieldVariant,
    selection_variant: &'a ScalarSelectionDirectiveSet,
) -> Option<Loadability<'a>> {
    match variant {
        ClientFieldVariant::Link => None,
        ClientFieldVariant::UserWritten(_) => match selection_variant {
            ScalarSelectionDirectiveSet::None(_) => None,
            ScalarSelectionDirectiveSet::Updatable(_) => None,
            ScalarSelectionDirectiveSet::Loadable(l) => {
                Some(Loadability::LoadablySelectedField(&l.loadable))
            }
        },
        ClientFieldVariant::ImperativelyLoadedField(i) => {
            Some(Loadability::ImperativelyLoadedField(i))
        }
    }
}
