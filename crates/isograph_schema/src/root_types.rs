use common_lang_types::{ObjectTypeName, WithLocation};
use isograph_lang_types::ObjectId;

pub(crate) struct RootTypes<T> {
    pub(crate) query: Option<T>,
    pub(crate) mutation: Option<T>,
    pub(crate) subscription: Option<T>,
}
pub(crate) type EncounteredRootTypes = RootTypes<ObjectId>;
pub(crate) type ProcessedRootTypes = RootTypes<WithLocation<ObjectTypeName>>;
