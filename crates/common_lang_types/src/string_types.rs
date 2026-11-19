use serde::Serialize;

#[derive(Clone, Debug, Serialize, PartialEq, Eq, PartialOrd, Ord, Hash, derive_more::Display)]
pub struct QueryText(pub String);

#[derive(Debug, Serialize, derive_more::Display)]
pub struct QueryExtraInfo(pub String);
