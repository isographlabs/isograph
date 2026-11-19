use isograph_compiler::ArtifactPath;

use prelude::Postfix;

use crate::imports::imports_of;

/// A module whose only export is its default export.
#[derive(Clone, Debug, PartialEq)]
pub struct ArtifactModule {
    pub path: ArtifactPath,
    pub default_export: ArtifactValue,
    pub default_export_type: DefaultExportType,
}

/// Whether the default export is declared with a type.
#[derive(Clone, Debug, PartialEq)]
pub enum DefaultExportType {
    /// `export default <value>;`: TypeScript infers the type.
    Inferred,
    /// `const <binding>: <type> = <value>; export default <binding>;`
    Declared(DeclaredExportType),
}

/// `<type_import>`, or `<type_import><type_arguments>` when it has arguments.
#[derive(Clone, Debug, PartialEq)]
pub struct DeclaredExportType {
    /// The name of the `const` that holds the value.
    pub binding: String,
    pub type_import: NamedTypeImport,
    pub type_arguments: Vec<DeclaredTypeArgument>,
}

/// A type argument of a declared export type.
#[derive(Clone, Debug, PartialEq)]
pub enum DeclaredTypeArgument {
    /// A type imported by name; the module imports it.
    Import(NamedTypeImport),
    /// A string literal type.
    StringLiteral(String),
}

/// A value a module exports, in no particular host language.
#[derive(Clone, Debug, PartialEq)]
pub enum ArtifactValue {
    Null,
    Boolean(bool),
    Number(f64),
    String(String),
    Array(Vec<ArtifactValue>),
    Object(Vec<ArtifactObjectEntry>),
    Import(ArtifactImport),
}

#[derive(Clone, Debug, PartialEq)]
pub struct ArtifactObjectEntry {
    pub key: String,
    pub value: ArtifactValue,
}

/// The default export of the module at this path.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ArtifactImport(pub ArtifactPath);

/// A TypeScript module: its path and its items, in order.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct TypeScriptModule {
    pub path: ArtifactPath,
    pub items: Vec<TypeScriptItem>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum TypeScriptItem {
    /// `import <identifier> from "<specifier>";`
    DefaultImport(ArtifactImport),
    /// `import type <identifier> from "<specifier>";`
    TypeImport(ArtifactImport),
    /// `import type { <imported> as <local> } from "<specifier>";`, or `{ <imported> }` when
    /// the two names are equal.
    NamedTypeImport(NamedTypeImport),
    /// `type <name><params> = <type>;`
    TypeAlias(TypeAlias),
    /// `export type <name><params> = <type>;`
    ExportedTypeAlias(TypeAlias),
    /// `export function <name><params>(<params>): <type>;`
    ExportedOverload(FunctionSignature),
    /// `export function <name><params>(<params>): <type> { throw new Error("<message>"); }`
    ExportedThrowingFunction(ThrowingFunction),
    /// `export default <value>;`
    ExportDefault(ArtifactValue),
    /// `const <binding>: <type> = <value>;`
    DeclaredConst(DeclaredConst),
    /// `export default <binding>;`
    ExportDefaultBinding(String),
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct DeclaredConst {
    pub binding: String,
    pub type_: TypeScriptType,
    pub value: ArtifactValue,
}

/// `import type { <imported> as <local> } from "<specifier>";`, or `{ <imported> }` when the
/// two names are equal.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct NamedTypeImport {
    /// Printed as written.
    pub specifier: String,
    pub imported: String,
    pub local: String,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct TypeAlias {
    pub name: String,
    pub type_params: Vec<TypeParam>,
    pub aliased: TypeScriptType,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct TypeParam {
    pub name: String,
    pub constraint: Option<TypeScriptType>,
    pub default: Option<TypeScriptType>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct FunctionSignature {
    pub name: String,
    pub type_params: Vec<TypeParam>,
    pub params: Vec<FunctionParam>,
    pub return_type: TypeScriptType,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct FunctionParam {
    pub name: String,
    pub type_: TypeScriptType,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ThrowingFunction {
    pub signature: FunctionSignature,
    pub message: String,
}

/// A TypeScript type.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum TypeScriptType {
    Keyword(TypeScriptKeyword),
    /// `Name<Args>`, or `Name` with no arguments.
    Reference(TypeReference),
    StringLiteral(String),
    TemplateLiteral(TemplateLiteralType),
    Union(Vec<TypeScriptType>),
    Intersection(Vec<TypeScriptType>),
    Conditional(Box<ConditionalType>),
    /// `infer Name`
    Infer(String),
    /// `typeof <identifier>`
    TypeOf(String),
    Function(Box<FunctionType>),
    /// `{ readonly <key>: <type>; ... }`
    Object(Vec<PropertySignature>),
    /// `(<type>)`
    Parenthesized(Box<TypeScriptType>),
}

/// `readonly <key>: <type>` or `readonly <key>?: <type>`, after the description as a JSDoc
/// comment.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PropertySignature {
    /// An identifier.
    pub key: String,
    pub optionality: Optionality,
    pub type_: TypeScriptType,
    pub description: Option<String>,
}

/// Whether a property may be absent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Optionality {
    Required,
    Optional,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TypeScriptKeyword {
    String,
    Number,
    Boolean,
    Null,
    Any,
    Unknown,
    Never,
    Void,
    Object,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct TypeReference {
    pub name: String,
    pub args: Vec<TypeScriptType>,
}

/// `` `<head>${<type>}<text>${<type>}<text>` ``
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct TemplateLiteralType {
    pub head: String,
    pub spans: Vec<TemplateLiteralSpan>,
}

/// A substituted type and the text after it.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct TemplateLiteralSpan {
    pub type_: TypeScriptType,
    pub text: String,
}

/// `<check> extends <extends> ? <true> : <false>`
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ConditionalType {
    pub check: TypeScriptType,
    pub extends: TypeScriptType,
    pub true_type: TypeScriptType,
    pub false_type: TypeScriptType,
}

/// `<params>(<params>) => <return>`
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct FunctionType {
    pub type_params: Vec<TypeParam>,
    pub params: Vec<FunctionParam>,
    pub return_type: TypeScriptType,
}

/// `artifact` as a TypeScript module. Inferred: a default import for each distinct import in
/// its value, in path order, then the default export. Declared: the type's imports, the value's
/// default imports, the `const` declared with the type, then its default export.
pub(crate) fn module_of_artifact(artifact: &ArtifactModule) -> TypeScriptModule {
    let default_imports = imports_of(&artifact.default_export)
        .into_iter()
        .cloned()
        .map(TypeScriptItem::DefaultImport);
    let items = match artifact.default_export_type.reference() {
        DefaultExportType::Inferred => default_imports
            .chain(std::iter::once(TypeScriptItem::ExportDefault(
                artifact.default_export.clone(),
            )))
            .collect(),
        DefaultExportType::Declared(declared) => std::iter::once(TypeScriptItem::NamedTypeImport(
            declared.type_import.clone(),
        ))
        .chain(
            declared
                .type_arguments
                .iter()
                .filter_map(|argument| match argument {
                    DeclaredTypeArgument::Import(import) => {
                        TypeScriptItem::NamedTypeImport(import.clone()).wrap_some()
                    }
                    DeclaredTypeArgument::StringLiteral(_) => None,
                }),
        )
        .chain(default_imports)
        .chain([
            TypeScriptItem::DeclaredConst(DeclaredConst {
                binding: declared.binding.clone(),
                type_: TypeScriptType::generic(
                    declared.type_import.local.as_str(),
                    declared
                        .type_arguments
                        .iter()
                        .map(|argument| match argument {
                            DeclaredTypeArgument::Import(import) => {
                                TypeScriptType::reference(import.local.as_str())
                            }
                            DeclaredTypeArgument::StringLiteral(literal) => {
                                TypeScriptType::StringLiteral(literal.clone())
                            }
                        })
                        .collect(),
                ),
                value: artifact.default_export.clone(),
            }),
            TypeScriptItem::ExportDefaultBinding(declared.binding.clone()),
        ])
        .collect(),
    };
    TypeScriptModule {
        path: artifact.path.clone(),
        items,
    }
}

impl TypeScriptType {
    pub(crate) fn reference(name: &str) -> Self {
        Self::Reference(TypeReference {
            name: name.to_owned(),
            args: Vec::new(),
        })
    }

    pub(crate) fn generic(name: &str, args: Vec<TypeScriptType>) -> Self {
        Self::Reference(TypeReference {
            name: name.to_owned(),
            args,
        })
    }
}
