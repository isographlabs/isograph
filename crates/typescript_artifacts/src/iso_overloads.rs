use std::cmp::Ordering;
use std::collections::BTreeSet;

use common_lang_types::EntityNameAndSelectableName;
use isograph_compiler::{ArtifactPath, ArtifactSource};
use prelude::Postfix;

use crate::entrypoint::entrypoint_path;
use crate::module::{
    ArtifactImport, ConditionalType, FunctionParam, FunctionSignature, FunctionType,
    FunctionTypeParam, TemplateLiteralSpan, TemplateLiteralType, ThrowingFunction, TypeAlias,
    TypeParam, TypeScriptItem, TypeScriptKeyword, TypeScriptModule, TypeScriptType,
};

const ISO_TS: &str = "iso.ts";

const UNEXPECTED_INVOCATION: &str = "iso: Unexpected invocation at runtime. Either the Babel \
    transform was not set up, or it failed to identify this call site. Make sure it is being \
    used verbatim as `iso`.";

/// `iso.ts`: an `iso` overload per field, entity, and entrypoint literal, and the
/// implementation.
pub(crate) fn iso_overload_module(sources: &BTreeSet<ArtifactSource>) -> TypeScriptModule {
    let fields = sorted(sources.iter().filter_map(|source| match source {
        ArtifactSource::Field(field) => field_overload_text(field.dereference()).wrap_some(),
        ArtifactSource::Entity(_) | ArtifactSource::Entrypoint(_) => None,
    }));
    let entities = sorted(sources.iter().filter_map(|source| match source {
        ArtifactSource::Entity(entity) => format!("entity {entity}").wrap_some(),
        ArtifactSource::Field(_) | ArtifactSource::Entrypoint(_) => None,
    }));
    let mut entrypoints: Vec<(String, ArtifactImport)> = sources
        .iter()
        .filter_map(|source| match source {
            ArtifactSource::Entrypoint(entrypoint) => (
                entrypoint_overload_text(entrypoint.dereference()),
                ArtifactImport(entrypoint_path(entrypoint.dereference())),
            )
                .wrap_some(),
            ArtifactSource::Entity(_) | ArtifactSource::Field(_) => None,
        })
        .collect();
    entrypoints.sort_by(|(a, _), (b, _)| overload_order(a, b));

    let imports = entrypoints
        .iter()
        .map(|(_, import)| TypeScriptItem::TypeImport(import.clone()));
    let field_overloads = fields
        .into_iter()
        .map(|text| overload(text, TypeScriptType::reference("Identity")));
    let entity_overloads = entities
        .into_iter()
        .map(|text| overload(text, TypeScriptType::Keyword(TypeScriptKeyword::Void)));
    let entrypoint_overloads = entrypoints
        .iter()
        .map(|(text, import)| overload(text.clone(), TypeScriptType::TypeOf(import.clone())));
    TypeScriptModule {
        path: ArtifactPath(ISO_TS.to_owned()),
        items: imports
            .chain(preamble())
            .chain(field_overloads)
            .chain(entity_overloads)
            .chain(entrypoint_overloads)
            .chain(std::iter::once(implementation()))
            .collect(),
    }
}

fn field_overload_text(field: EntityNameAndSelectableName) -> String {
    format!(
        "field {}.{}",
        field.parent_entity_name, field.selectable_name
    )
}

fn entrypoint_overload_text(entrypoint: EntityNameAndSelectableName) -> String {
    format!(
        "entrypoint {}.{}",
        entrypoint.parent_entity_name, entrypoint.selectable_name
    )
}

fn sorted(texts: impl Iterator<Item = String>) -> Vec<String> {
    let mut texts: Vec<String> = texts.collect();
    texts.sort_by(|a, b| overload_order(a, b));
    texts
}

/// Upstream's `sort_field_name` rule over the whole match text: a string sorts before every
/// string it is a prefix of, so a shorter match text cannot shadow a longer one.
pub(crate) fn overload_order(a: &str, b: &str) -> Ordering {
    let key = |text: &str| {
        text.bytes()
            .map(u16::from)
            .chain(std::iter::once(256))
            .collect::<Vec<_>>()
    };
    key(a).cmp(key(b).reference())
}

fn t() -> TypeScriptType {
    TypeScriptType::reference("T")
}

/// `export function iso<T>(param: T & MatchesWhitespaceAndString<"<text>", T>): <returns>;`
fn overload(text: String, returns: TypeScriptType) -> TypeScriptItem {
    TypeScriptItem::ExportedOverload(FunctionSignature {
        name: "iso".to_owned(),
        type_params: TypeParam {
            name: "T".to_owned(),
            constraint: None,
        }
        .wrap_vec(),
        params: FunctionParam {
            name: "param".to_owned(),
            type_: TypeScriptType::Intersection(vec![
                t(),
                TypeScriptType::generic(
                    "MatchesWhitespaceAndString",
                    vec![TypeScriptType::StringLiteral(text), t()],
                ),
            ]),
        }
        .wrap_vec(),
        return_type: returns,
    })
}

fn implementation() -> TypeScriptItem {
    TypeScriptItem::ExportedThrowingFunction(ThrowingFunction {
        signature: FunctionSignature {
            name: "iso".to_owned(),
            type_params: Vec::new(),
            params: FunctionParam {
                name: "_isographLiteralText".to_owned(),
                type_: TypeScriptType::Keyword(TypeScriptKeyword::String),
            }
            .wrap_vec(),
            return_type: TypeScriptType::Union(vec![
                TypeScriptType::reference("Identity"),
                TypeScriptType::Keyword(TypeScriptKeyword::Object),
                TypeScriptType::Keyword(TypeScriptKeyword::Void),
            ]),
        },
        message: UNEXPECTED_INVOCATION.to_owned(),
    })
}

/// `Identity`, `WhitespaceCharacter`, `Whitespace`, and `MatchesWhitespaceAndString`.
fn preamble() -> impl Iterator<Item = TypeScriptItem> {
    let client_field = || TypeScriptType::reference("TClientField");
    let identity = TypeAlias {
        name: "Identity".to_owned(),
        type_params: Vec::new(),
        aliased: TypeScriptType::Function(
            FunctionType {
                type_params: TypeParam {
                    name: "TClientField".to_owned(),
                    constraint: TypeScriptType::Function(
                        FunctionType {
                            type_params: Vec::new(),
                            params: FunctionTypeParam::Rest(FunctionParam {
                                name: "args".to_owned(),
                                type_: TypeScriptType::Array(
                                    TypeScriptType::Keyword(TypeScriptKeyword::Never).boxed(),
                                ),
                            })
                            .wrap_vec(),
                            return_type: TypeScriptType::Keyword(TypeScriptKeyword::Unknown),
                        }
                        .boxed(),
                    )
                    .wrap_some(),
                }
                .wrap_vec(),
                params: FunctionTypeParam::Named(FunctionParam {
                    name: "clientField".to_owned(),
                    type_: client_field(),
                })
                .wrap_vec(),
                return_type: client_field(),
            }
            .boxed(),
        ),
    };
    let whitespace_character = TypeAlias {
        name: "WhitespaceCharacter".to_owned(),
        type_params: Vec::new(),
        aliased: TypeScriptType::Union(
            [" ", "\t", "\n"]
                .into_iter()
                .map(|character| TypeScriptType::StringLiteral(character.to_owned()))
                .collect(),
        ),
    };
    let whitespace = TypeAlias {
        name: "Whitespace".to_owned(),
        type_params: TypeParam {
            name: "In".to_owned(),
            constraint: None,
        }
        .wrap_vec(),
        aliased: TypeScriptType::Conditional(
            ConditionalType {
                check: TypeScriptType::reference("In"),
                extends: TypeScriptType::TemplateLiteral(TemplateLiteralType {
                    head: String::new(),
                    spans: vec![
                        TemplateLiteralSpan {
                            type_: TypeScriptType::reference("WhitespaceCharacter"),
                            text: String::new(),
                        },
                        TemplateLiteralSpan {
                            type_: TypeScriptType::Infer("In".to_owned()),
                            text: String::new(),
                        },
                    ],
                }),
                true_type: TypeScriptType::generic(
                    "Whitespace",
                    TypeScriptType::reference("In").wrap_vec(),
                ),
                false_type: TypeScriptType::reference("In"),
            }
            .boxed(),
        ),
    };
    let matches = TypeAlias {
        name: "MatchesWhitespaceAndString".to_owned(),
        type_params: vec![
            TypeParam {
                name: "TString".to_owned(),
                constraint: TypeScriptType::Keyword(TypeScriptKeyword::String).wrap_some(),
            },
            TypeParam {
                name: "T".to_owned(),
                constraint: None,
            },
        ],
        aliased: TypeScriptType::Conditional(
            ConditionalType {
                check: TypeScriptType::generic("Whitespace", t().wrap_vec()),
                extends: TypeScriptType::TemplateLiteral(TemplateLiteralType {
                    head: String::new(),
                    spans: vec![
                        TemplateLiteralSpan {
                            type_: TypeScriptType::reference("TString"),
                            text: String::new(),
                        },
                        TemplateLiteralSpan {
                            type_: TypeScriptType::Keyword(TypeScriptKeyword::String),
                            text: String::new(),
                        },
                    ],
                }),
                true_type: t(),
                false_type: TypeScriptType::Keyword(TypeScriptKeyword::Never),
            }
            .boxed(),
        ),
    };
    [identity, whitespace_character, whitespace, matches]
        .into_iter()
        .map(TypeScriptItem::TypeAlias)
}
