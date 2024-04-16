use common_lang_types::{GraphQLObjectTypeName, WithLocation};
use graphql_lang_types::RootOperationKind;
use isograph_lang_types::ServerObjectId;

pub struct RootTypes<T> {
    pub query: Option<T>,
    pub mutation: Option<T>,
    pub subscription: Option<T>,
}

impl<T> RootTypes<T> {
    pub fn set_root_type(&mut self, root_kind: RootOperationKind, value: T) {
        match root_kind {
            RootOperationKind::Query => {
                if self.query.is_some() {
                    panic!(
                        "Unexpected redefinition of RootTypes.query. This is \
                        indicative of a bug in Isograph"
                    );
                }
                self.query = Some(value);
            }
            RootOperationKind::Subscription => {
                if self.subscription.is_some() {
                    panic!(
                        "Unexpected redefinition of RootTypes.subscription. This is \
                        indicative of a bug in Isograph"
                    );
                }
                self.subscription = Some(value);
            }
            RootOperationKind::Mutation => {
                if self.mutation.is_some() {
                    panic!(
                        "Unexpected redefinition of RootTypes.mutation. This is \
                        indicative of a bug in Isograph"
                    );
                }
                self.mutation = Some(value);
            }
        }
    }
}

pub type EncounteredRootTypes = RootTypes<ServerObjectId>;
pub type ProcessedRootTypes = RootTypes<WithLocation<GraphQLObjectTypeName>>;
