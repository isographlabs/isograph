use common_lang_types::Diagnostic;
use prelude::Postfix;

use crate::{CompilationProfile, IsographDatabase};

// We should consider making CreateError generic over TKey, and have
// DataModelXYZ implement a trait that returns a key, because as this
// is designed, we must store the key redundantly inside of the
// CreateError. Keys are cheap, at least until we get to nested
// selection sets, so this is a problem for the future.
pub trait CreateError: Clone {
    fn create_error<TCompilationProfile: CompilationProfile>(
        self,
        db: &IsographDatabase<TCompilationProfile>,
    ) -> Diagnostic;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LazyValidation<TItem, TCreateError: CreateError> {
    value: Result<TItem, TCreateError>,
}

impl<TItem, TCreateError: CreateError> LazyValidation<TItem, TCreateError> {
    pub fn new(value: Result<TItem, TCreateError>) -> Self {
        Self { value }
    }

    pub fn as_ref(&self) -> LazyValidation<&TItem, TCreateError> {
        LazyValidation::new(match self.value.reference() {
            Ok(v) => v.wrap_ok(),
            Err(e) => e.clone().wrap_err(),
        })
    }

    pub fn map<U>(self, map: impl FnOnce(TItem) -> U) -> LazyValidation<U, TCreateError> {
        LazyValidation::new(self.value.map(map))
    }

    pub fn to_result<TCompilationProfile: CompilationProfile>(
        self,
        db: &IsographDatabase<TCompilationProfile>,
    ) -> Result<TItem, Diagnostic> {
        self.value.map_err(|e| e.create_error(db))
    }
}
