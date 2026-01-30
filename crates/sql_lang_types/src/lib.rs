use sqlparser::ast::Statement;

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Default, Hash)]
pub struct SQLTypeSystemDocument(pub Vec<Statement>);
