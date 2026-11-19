use std::cmp::Ordering;

use common_lang_types::EntityNameAndSelectableName;
use isograph_compiler::{ArtifactPath, ClientFieldKind, FieldArtifactKind, ProjectArtifacts};
use prelude::Postfix;

use crate::entrypoint::{entrypoint_path, field_artifact_path};
use crate::generate_types::{
    PARAM_TYPE, link_type_name, link_type_path, param_type_name, record_of_never, sorted_imports,
    wrapped_type,
};
use crate::imports::{artifact_import_identifier, relative_artifact_specifier};
use crate::module::{
    ArtifactImport, ConditionalType, FunctionParam, FunctionSignature, FunctionType,
    NamedTypeImport, TemplateLiteralSpan, TemplateLiteralType, ThrowingFunction, TypeAlias,
    TypeParam, TypeScriptItem, TypeScriptKeyword, TypeScriptModule, TypeScriptType,
};

const ISO_TS: &str = "iso.ts";

const UNEXPECTED_INVOCATION: &str = "iso: Unexpected invocation at runtime. Either the Babel \
    transform was not set up, or it failed to identify this call site. Make sure it is being \
    used verbatim as `iso`.";

/// `iso.ts`: an `iso` overload per field, entity, and entrypoint literal, and the
/// implementation.
pub(crate) fn iso_overload_module<TLiteralContext>(
    project: &ProjectArtifacts<TLiteralContext>,
) -> TypeScriptModule {
    let path = ArtifactPath(ISO_TS.to_owned());
    let mut fields: Vec<(String, TypeScriptType)> = project
        .fields
        .iter()
        .map(|field| {
            (
                field_overload_text(field.field),
                field_overload_return(field.field, field.kind.reference()),
            )
        })
        .collect();
    fields.sort_by(|(a, _), (b, _)| overload_order(a, b));
    let entities = sorted(
        project
            .entities
            .iter()
            .map(|entity| format!("entity {entity}")),
    );
    let mut entrypoints: Vec<(String, ArtifactImport)> = project
        .entrypoints
        .iter()
        .map(|entrypoint| {
            (
                entrypoint_overload_text(entrypoint.dereference()),
                ArtifactImport(entrypoint_path(entrypoint.dereference())),
            )
        })
        .collect();
    entrypoints.sort_by(|(a, _), (b, _)| overload_order(a, b));

    let param_imports = sorted_imports(project.fields.iter().map(|field| {
        let name = param_type_name(field.field);
        NamedTypeImport {
            specifier: relative_artifact_specifier(
                path.reference(),
                field_artifact_path(field.field, PARAM_TYPE).reference(),
            ),
            imported: name.clone(),
            local: name,
        }
    }));
    let link_imports = sorted_imports(project.link_types.iter().map(|link| {
        let name = link_type_name(link.entity);
        NamedTypeImport {
            specifier: relative_artifact_specifier(
                path.reference(),
                link_type_path(link.entity).reference(),
            ),
            imported: name.clone(),
            local: name,
        }
    }));
    let entrypoint_imports = entrypoints
        .iter()
        .map(|(_, import)| TypeScriptItem::TypeImport(import.clone()));
    let field_overloads = fields
        .into_iter()
        .map(|(text, returns)| overload(text, returns));
    let entity_overloads = entities
        .into_iter()
        .map(|text| overload(text, TypeScriptType::Keyword(TypeScriptKeyword::Void)));
    let entrypoint_overloads = entrypoints.iter().map(|(text, import)| {
        overload(
            text.clone(),
            TypeScriptType::TypeOf(artifact_import_identifier(import)),
        )
    });
    TypeScriptModule {
        items: param_imports
            .chain(link_imports)
            .chain(entrypoint_imports)
            .chain(preamble())
            .chain(field_overloads)
            .chain(entity_overloads)
            .chain(entrypoint_overloads)
            .chain(std::iter::once(implementation()))
            .collect(),
        path,
    }
}

/// `IdentityWithParam<P__f__param>` for a plain client field,
/// `IdentityWithParamComponent<P__f__param>` for a component, and
/// `IdentityWithParam<P__f__param, T>` for a pointer, where `T` is the pointer's target with
/// each entity's link type.
fn field_overload_return(
    field: EntityNameAndSelectableName,
    kind: &FieldArtifactKind,
) -> TypeScriptType {
    let param = TypeScriptType::reference(param_type_name(field).as_str());
    match kind {
        FieldArtifactKind::ClientField(ClientFieldKind::Plain) => {
            TypeScriptType::generic(IDENTITY_WITH_PARAM, param.wrap_vec())
        }
        FieldArtifactKind::ClientField(ClientFieldKind::Component) => {
            TypeScriptType::generic(IDENTITY_WITH_PARAM_COMPONENT, param.wrap_vec())
        }
        FieldArtifactKind::Pointer(target) => TypeScriptType::generic(
            IDENTITY_WITH_PARAM,
            vec![
                param,
                wrapped_type(target, &mut |entity| {
                    TypeScriptType::reference(link_type_name(entity.dereference()).as_str())
                }),
            ],
        ),
    }
}

const IDENTITY_WITH_PARAM: &str = "IdentityWithParam";
const IDENTITY_WITH_PARAM_COMPONENT: &str = "IdentityWithParamComponent";

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
        type_params: type_param("T").wrap_vec(),
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
                TypeScriptType::generic(IDENTITY_WITH_PARAM, any().wrap_vec()),
                TypeScriptType::generic(IDENTITY_WITH_PARAM_COMPONENT, any().wrap_vec()),
                TypeScriptType::Keyword(TypeScriptKeyword::Object),
                TypeScriptType::Keyword(TypeScriptKeyword::Void),
            ]),
        },
        message: UNEXPECTED_INVOCATION.to_owned(),
    })
}

fn any() -> TypeScriptType {
    TypeScriptType::Keyword(TypeScriptKeyword::Any)
}

fn type_param(name: &str) -> TypeParam {
    TypeParam {
        name: name.to_owned(),
        constraint: None,
        default: None,
    }
}

fn named(name: &str, type_: TypeScriptType) -> FunctionParam {
    FunctionParam {
        name: name.to_owned(),
        type_,
    }
}

/// `(<params>) => <return>` with no type parameters.
fn function(params: Vec<FunctionParam>, return_type: TypeScriptType) -> TypeScriptType {
    TypeScriptType::Function(
        FunctionType {
            type_params: Vec::new(),
            params,
            return_type,
        }
        .boxed(),
    )
}

/// Upstream's `IdentityWithParam` and `IdentityWithParamComponent`, then
/// `WhitespaceCharacter`, `Whitespace`, and `MatchesWhitespaceAndString`.
fn preamble() -> impl Iterator<Item = TypeScriptItem> {
    let t_param = || TypeScriptType::reference("TParam");
    let t_return = || TypeScriptType::reference("TClientFieldReturn");
    let t_props = || TypeScriptType::reference("TComponentProps");
    let object_param = || TypeParam {
        name: "TParam".to_owned(),
        constraint: TypeScriptType::Keyword(TypeScriptKeyword::Object).wrap_some(),
        default: None,
    };
    let param_function = || function(named("param", t_param()).wrap_vec(), t_return());
    let identity = TypeAlias {
        name: IDENTITY_WITH_PARAM.to_owned(),
        type_params: vec![
            object_param(),
            TypeParam {
                name: "TReturnConstraint".to_owned(),
                constraint: None,
                default: TypeScriptType::Keyword(TypeScriptKeyword::Unknown).wrap_some(),
            },
        ],
        aliased: TypeScriptType::Function(
            FunctionType {
                type_params: TypeParam {
                    name: "TClientFieldReturn".to_owned(),
                    constraint: TypeScriptType::reference("TReturnConstraint").wrap_some(),
                    default: None,
                }
                .wrap_vec(),
                params: named("clientField", param_function()).wrap_vec(),
                return_type: param_function(),
            }
            .boxed(),
        ),
    };
    let component_function = || {
        function(
            vec![named("data", t_param()), named("componentProps", t_props())],
            t_return(),
        )
    };
    let identity_component = TypeAlias {
        name: IDENTITY_WITH_PARAM_COMPONENT.to_owned(),
        type_params: object_param().wrap_vec(),
        aliased: TypeScriptType::Function(
            FunctionType {
                type_params: vec![
                    type_param("TClientFieldReturn"),
                    TypeParam {
                        name: "TComponentProps".to_owned(),
                        constraint: None,
                        default: record_of_never().wrap_some(),
                    },
                ],
                params: named("clientComponentField", component_function()).wrap_vec(),
                return_type: component_function(),
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
        type_params: type_param("In").wrap_vec(),
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
                default: None,
            },
            type_param("T"),
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
    [
        identity,
        identity_component,
        whitespace_character,
        whitespace,
        matches,
    ]
    .into_iter()
    .map(TypeScriptItem::TypeAlias)
}
