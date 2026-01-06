use std::{convert::Infallible, fmt::Debug, hash::Hash};

use common_lang_types::{Diagnostic, EmbeddedLocation, SelectableName};

use crate::{CompilationProfile, MapWithNonfatalDiagnostics, NestedDataModelSelectable};

/// A trait that identifies how far along in the "transformation pipeline"
/// a data model item is. It is simply a bag of associated types.
/// We use this trait because we don't want to pass many type params
/// around, and because we want the type params to change together.
///
/// - When we parse the type system documents via [`NetworkProtocol`],
///   we return a `DataModelSchema<TNetworkProtocol, NestedState>`. At this
///   point, everything is nested, i.e. entities contain selectables, which
///   contain arguments, etc.
/// - The compiler will flatten this, i.e. convert it to
///   `DataModelSchema<TNetworkProtocol, FlattenedState>`. At this point,
///   nothing is nested, and nothing is validated, and there is no location
///   info.
/// - Before creating artifact text, the compiler will validate everything,
///   creating a
///   `Result<DataModelSchema<TNetworkProtocol, ValidatedState>, Vec<Diagnostic>>`.
///
/// For now, we will start with everything below the level of selectable
/// being nested, i.e. selectables will contain arguments. Once we have adopted
/// this pattern, we will flatten arguments, selection sets, etc.
pub trait DataModelStage:
    Copy + Clone + Debug + PartialEq + PartialOrd + Eq + Ord + Hash + Default
{
    type Error;
    type Location;

    type Selectables<TCompilationProfile: CompilationProfile>;
    // Arguments, SelectionSets, etc.
}

#[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Eq, Ord, Hash, Default)]
pub struct NestedStage {}

/// Initially, we have errors and locations.
impl DataModelStage for NestedStage {
    type Error = Diagnostic;
    type Location = Option<EmbeddedLocation>;
    // TODO WithGenericLocation<NestedDataModelSelectables>
    type Selectables<TCompilationProfile: CompilationProfile> = MapWithNonfatalDiagnostics<
        SelectableName,
        NestedDataModelSelectable<TCompilationProfile>,
        Self::Error,
    >;
}

/// Next, those locations are dropped and errors ignored.
/// In order to print those errors, we must go back to the original nested object and access
/// the error.
#[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Eq, Ord, Hash, Default)]
pub struct FlattenedStage {}
impl DataModelStage for FlattenedStage {
    type Error = ();
    type Location = ();
    type Selectables<TCompilationProfile: CompilationProfile> = ();
}

/// Finally, those errors are proven to not exist.
#[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Eq, Ord, Hash, Default)]
pub struct ValidatedStage {}
impl DataModelStage for ValidatedStage {
    type Error = Infallible;
    type Location = ();
    type Selectables<TCompilationProfile: CompilationProfile> = ();
}
