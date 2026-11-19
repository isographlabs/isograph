/// The TypeScript type of a custom scalar's values: a named type, imported from a module
/// when `import` is `Some`, otherwise a global or keyword type (`string`, `Date`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypeScriptCustomScalarType {
    pub name: &'static str,
    pub import: Option<ModuleSpecifier>,
}

/// A module specifier, printed as written in every artifact that imports from it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ModuleSpecifier(pub &'static str);
