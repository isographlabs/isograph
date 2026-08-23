use serde::Serialize;

#[derive(Debug, Serialize, PartialEq, Eq, PartialOrd, Ord, derive_more::Display)]
pub struct QueryText(pub String);

#[derive(Debug, Serialize, derive_more::Display)]
pub struct QueryExtraInfo(pub String);
