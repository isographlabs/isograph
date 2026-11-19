use std::collections::BTreeSet;
use std::path::{Component, Path};

use common_lang_types::{
    ConstExportName, EntityName, EntityNameAndSelectableName, RelativePathToSourceFile,
};
use intern::Lookup;
use isograph_compiler::{
    ArtifactInnerType, ArtifactInputType, ArtifactObjectType, ArtifactPath, FieldArtifacts,
    GenerationOptions, LinkType, Nullability, ParametersType, ScalarKind, ScalarValueKind, Wrapper,
    WrapperVariant,
};
use prelude::Postfix;

use crate::TypeScriptGenerationError;
use crate::custom_scalar::TypeScriptCustomScalarType;
use crate::entrypoint::field_artifact_path;
use crate::imports::relative_artifact_specifier;
use crate::module::{
    NamedTypeImport, Optionality, PropertySignature, TypeAlias, TypeScriptItem, TypeScriptKeyword,
    TypeScriptModule, TypeScriptType,
};

pub(crate) type Options = GenerationOptions<TypeScriptCustomScalarType>;

pub(crate) const PARAM_TYPE: &str = "param_type.ts";
const PARAMETERS_TYPE: &str = "parameters_type.ts";
const OUTPUT_TYPE: &str = "output_type.ts";

/// `P__f__param`.
pub(crate) fn param_type_name(field: EntityNameAndSelectableName) -> String {
    format!("{}__param", field.underscore_separated())
}

fn parameters_type_name(field: EntityNameAndSelectableName) -> String {
    format!("{}__parameters", field.underscore_separated())
}

fn output_type_name(field: EntityNameAndSelectableName) -> String {
    format!("{}__output_type", field.underscore_separated())
}

/// `E____link__output_type`.
pub(crate) fn link_type_name(entity: EntityName) -> String {
    format!("{entity}____link__output_type")
}

pub(crate) fn link_type_path(entity: EntityName) -> ArtifactPath {
    ArtifactPath(format!("{entity}/__link/output_type.ts"))
}

/// `P/f/param_type.ts`.
pub(crate) fn param_type_file<TLiteralContext>(
    field: &FieldArtifacts<TLiteralContext>,
    options: &Options,
) -> TypeScriptModule {
    let path = field_artifact_path(field.field, PARAM_TYPE);
    let output_type_imports = object_inner_types(field.data.reference())
        .filter_map(|inner| match inner {
            ArtifactInnerType::ClientFieldOutput(selected) => selected.dereference().wrap_some(),
            ArtifactInnerType::Scalar(_) | ArtifactInnerType::Object(_) => None,
        })
        .collect::<BTreeSet<_>>()
        .into_iter()
        .map(|selected| {
            let name = output_type_name(selected);
            NamedTypeImport {
                specifier: relative_artifact_specifier(
                    path.reference(),
                    field_artifact_path(selected, OUTPUT_TYPE).reference(),
                ),
                imported: name.clone(),
                local: name,
            }
        });
    let custom_scalars = object_inner_types(field.data.reference())
        .filter_map(|inner| match inner {
            ArtifactInnerType::Scalar(ScalarKind::Custom(name)) => name.dereference().wrap_some(),
            ArtifactInnerType::Scalar(ScalarKind::Known(_))
            | ArtifactInnerType::Object(_)
            | ArtifactInnerType::ClientFieldOutput(_) => None,
        })
        .collect::<BTreeSet<_>>();
    let parameters_import = field.parameters.as_ref().map(|_| {
        let name = parameters_type_name(field.field);
        NamedTypeImport {
            specifier: relative_artifact_specifier(
                path.reference(),
                field_artifact_path(field.field, PARAMETERS_TYPE).reference(),
            ),
            imported: name.clone(),
            local: name,
        }
    });
    let parameters = match field.parameters {
        Some(_) => TypeScriptType::reference(parameters_type_name(field.field).as_str()),
        None => record_of_never(),
    };
    let param = TypeAlias {
        name: param_type_name(field.field),
        type_params: Vec::new(),
        aliased: TypeScriptType::Object(vec![
            required("data", object_type(field.data.reference(), options)),
            required("parameters", parameters),
        ]),
    };
    TypeScriptModule {
        items: sorted_imports(
            output_type_imports
                .chain(custom_scalar_imports(custom_scalars.reference(), options))
                .chain(parameters_import),
        )
        .chain(std::iter::once(TypeScriptItem::ExportedTypeAlias(param)))
        .collect(),
        path,
    }
}

/// `P/f/parameters_type.ts`.
pub(crate) fn parameters_type_file(
    field: EntityNameAndSelectableName,
    parameters: &ParametersType,
    options: &Options,
) -> TypeScriptModule {
    let input_types = parameters
        .parameters
        .iter()
        .map(|parameter| parameter.type_.reference())
        .chain(
            parameters
                .input_objects
                .values()
                .flatten()
                .map(|input_field| input_field.type_.reference()),
        );
    let custom_scalars = input_types
        .flat_map(|type_| type_.inner_values())
        .filter_map(|inner| match inner {
            ArtifactInputType::Scalar(ScalarKind::Custom(name)) => name.dereference().wrap_some(),
            ArtifactInputType::Scalar(ScalarKind::Known(_)) | ArtifactInputType::InputObject(_) => {
                None
            }
        })
        .collect::<BTreeSet<_>>();
    let exported = TypeAlias {
        name: parameters_type_name(field),
        type_params: Vec::new(),
        aliased: TypeScriptType::Object(
            parameters
                .parameters
                .iter()
                .map(|parameter| {
                    parameter_property(
                        parameter.name.lookup(),
                        parameter.type_.reference(),
                        options,
                    )
                })
                .collect(),
        ),
    };
    let input_objects = parameters.input_objects.iter().map(|(name, fields)| {
        TypeScriptItem::TypeAlias(TypeAlias {
            name: name.to_string(),
            type_params: Vec::new(),
            aliased: TypeScriptType::Object(
                fields
                    .iter()
                    .map(|input_field| {
                        parameter_property(
                            input_field.name.lookup(),
                            input_field.type_.reference(),
                            options,
                        )
                    })
                    .collect(),
            ),
        })
    });
    TypeScriptModule {
        path: field_artifact_path(field, PARAMETERS_TYPE),
        items: sorted_imports(custom_scalar_imports(custom_scalars.reference(), options))
            .chain(input_objects)
            .chain(std::iter::once(TypeScriptItem::ExportedTypeAlias(exported)))
            .collect(),
    }
}

/// `P/f/output_type.ts`: what the resolver exported as `export` from `source` returns.
pub(crate) fn output_type_file(
    field: EntityNameAndSelectableName,
    source: RelativePathToSourceFile,
    export: Option<ConstExportName>,
    options: &Options,
) -> Result<TypeScriptModule, TypeScriptGenerationError> {
    let path = field_artifact_path(field, OUTPUT_TYPE);
    let export = export.ok_or(TypeScriptGenerationError::MissingExport(field))?;
    let specifier = source_specifier(path.reference(), source, options)
        .ok_or(TypeScriptGenerationError::SourceNotRelative(field))?;
    TypeScriptModule {
        items: vec![
            TypeScriptItem::NamedTypeImport(NamedTypeImport {
                specifier,
                imported: export.to_string(),
                local: "resolver".to_owned(),
            }),
            TypeScriptItem::ExportedTypeAlias(TypeAlias {
                name: output_type_name(field),
                type_params: Vec::new(),
                aliased: TypeScriptType::generic(
                    "ReturnType",
                    TypeScriptType::TypeOf("resolver".to_owned()).wrap_vec(),
                ),
            }),
        ],
        path,
    }
    .wrap_ok()
}

/// `E/__link/output_type.ts`.
pub(crate) fn link_type_file(link: &LinkType) -> TypeScriptModule {
    let mut objects: Vec<TypeScriptType> = link
        .concrete_types
        .iter()
        .map(|concrete| {
            TypeScriptType::Object(vec![
                required("__link", TypeScriptType::Keyword(TypeScriptKeyword::String)),
                required(
                    "__typename",
                    TypeScriptType::StringLiteral(concrete.to_string()),
                ),
            ])
        })
        .collect();
    let aliased = match objects.len() {
        0 => TypeScriptType::Keyword(TypeScriptKeyword::Never),
        1 => objects.remove(0),
        _ => TypeScriptType::Union(objects),
    };
    TypeScriptModule {
        path: link_type_path(link.entity),
        items: TypeScriptItem::ExportedTypeAlias(TypeAlias {
            name: link_type_name(link.entity),
            type_params: Vec::new(),
            aliased,
        })
        .wrap_vec(),
    }
}

/// `wrapper` with `inner` printed at each `Inner`: `List` is `ReadonlyArray<T>`, a `Union`
/// of two or more variants is `(A | B)`, `Null` is `null`, a one-variant union is its variant.
pub(crate) fn wrapped_type<T>(
    wrapper: &Wrapper<T>,
    inner: &mut impl FnMut(&T) -> TypeScriptType,
) -> TypeScriptType {
    match wrapper {
        Wrapper::Inner(value) => inner(value),
        Wrapper::List(list) => readonly_array(wrapped_type(list, inner)),
        Wrapper::Union(union) => {
            let mut members: Vec<TypeScriptType> = union
                .0
                .iter()
                .map(|variant| variant_type(variant, inner))
                .collect();
            match members.len() {
                1 => members.remove(0),
                _ => TypeScriptType::Parenthesized(TypeScriptType::Union(members).boxed()),
            }
        }
    }
}

fn variant_type<T>(
    variant: &WrapperVariant<T>,
    inner: &mut impl FnMut(&T) -> TypeScriptType,
) -> TypeScriptType {
    match variant {
        WrapperVariant::Inner(value) => inner(value),
        WrapperVariant::List(list) => readonly_array(wrapped_type(list, inner)),
        WrapperVariant::Null => TypeScriptType::Keyword(TypeScriptKeyword::Null),
    }
}

fn readonly_array(element: TypeScriptType) -> TypeScriptType {
    TypeScriptType::generic("ReadonlyArray", element.wrap_vec())
}

/// A parameter's property: like `wrapped_type`, except that a top-level nullable union prints
/// its other variants then `null | void` with no parentheses (upstream's
/// `format_parameter_type`), and the property is optional.
fn parameter_property(
    key: &str,
    wrapper: &Wrapper<ArtifactInputType>,
    options: &Options,
) -> PropertySignature {
    let mut inner = |input: &ArtifactInputType| input_type(input, options);
    let (optionality, type_) = match (wrapper, wrapper.nullability()) {
        (Wrapper::Union(union), Nullability::Nullable) => (
            Optionality::Optional,
            TypeScriptType::Union(
                union
                    .0
                    .iter()
                    .filter(|variant| !matches!(variant, WrapperVariant::Null))
                    .map(|variant| variant_type(variant, &mut inner))
                    .chain([
                        TypeScriptType::Keyword(TypeScriptKeyword::Null),
                        TypeScriptType::Keyword(TypeScriptKeyword::Void),
                    ])
                    .collect(),
            ),
        ),
        (Wrapper::Inner(_) | Wrapper::List(_) | Wrapper::Union(_), _) => {
            (Optionality::Required, wrapped_type(wrapper, &mut inner))
        }
    };
    PropertySignature {
        key: key.to_owned(),
        optionality,
        type_,
        description: None,
    }
}

fn input_type(input: &ArtifactInputType, options: &Options) -> TypeScriptType {
    match input {
        ArtifactInputType::Scalar(scalar) => scalar_type(scalar.dereference(), options),
        ArtifactInputType::InputObject(name) => TypeScriptType::reference(name.lookup()),
    }
}

/// `{ readonly k: T; ... }`, each property with its description as a leading JSDoc comment.
fn object_type(object: &ArtifactObjectType, options: &Options) -> TypeScriptType {
    TypeScriptType::Object(
        object
            .0
            .iter()
            .map(|field| PropertySignature {
                key: field.key.to_string(),
                optionality: Optionality::Required,
                type_: wrapped_type(field.type_.reference(), &mut |inner| {
                    inner_type(inner, options)
                }),
                description: field.description.map(|description| description.to_string()),
            })
            .collect(),
    )
}

fn inner_type(inner: &ArtifactInnerType, options: &Options) -> TypeScriptType {
    match inner {
        ArtifactInnerType::Scalar(scalar) => scalar_type(scalar.dereference(), options),
        ArtifactInnerType::Object(object) => object_type(object, options),
        ArtifactInnerType::ClientFieldOutput(field) => {
            TypeScriptType::reference(output_type_name(field.dereference()).as_str())
        }
    }
}

/// Every inner type of `object`'s fields, at any depth.
fn object_inner_types(
    object: &ArtifactObjectType,
) -> Box<dyn Iterator<Item = &ArtifactInnerType> + '_> {
    object
        .0
        .iter()
        .flat_map(|field| field.type_.inner_values())
        .flat_map(
            |inner| -> Box<dyn Iterator<Item = &ArtifactInnerType> + '_> {
                match inner {
                    ArtifactInnerType::Object(nested) => std::iter::once(inner)
                        .chain(object_inner_types(nested))
                        .boxed(),
                    ArtifactInnerType::Scalar(_) | ArtifactInnerType::ClientFieldOutput(_) => {
                        std::iter::once(inner).boxed()
                    }
                }
            },
        )
        .boxed()
}

/// The TypeScript type of a scalar's values.
fn scalar_type(kind: ScalarKind, options: &Options) -> TypeScriptType {
    match kind {
        ScalarKind::Known(value_kind) => value_kind_type(value_kind),
        ScalarKind::Custom(name) => options
            .custom_scalar_types
            .get(name.reference())
            .map(|custom| custom_scalar_type(name, custom))
            .unwrap_or(TypeScriptType::Keyword(TypeScriptKeyword::Unknown)),
    }
}

fn value_kind_type(value_kind: ScalarValueKind) -> TypeScriptType {
    TypeScriptType::Keyword(match value_kind {
        ScalarValueKind::String => TypeScriptKeyword::String,
        ScalarValueKind::Number => TypeScriptKeyword::Number,
        ScalarValueKind::Boolean => TypeScriptKeyword::Boolean,
    })
}

/// A type reference to `custom.name` when it has no import, else to the import's local name
/// `<scalar>__custom_scalar`.
fn custom_scalar_type(scalar: EntityName, custom: &TypeScriptCustomScalarType) -> TypeScriptType {
    match custom.import {
        None => TypeScriptType::reference(custom.name),
        Some(_) => TypeScriptType::reference(custom_scalar_local_name(scalar).as_str()),
    }
}

fn custom_scalar_local_name(scalar: EntityName) -> String {
    format!("{scalar}__custom_scalar")
}

/// `import type { <name> as <scalar>__custom_scalar } from "<module>"` for each scalar in
/// `scalars` whose entry has an import.
fn custom_scalar_imports<'a>(
    scalars: &'a BTreeSet<EntityName>,
    options: &'a Options,
) -> impl Iterator<Item = NamedTypeImport> + 'a {
    scalars.iter().filter_map(|scalar| {
        let custom = options.custom_scalar_types.get(scalar)?;
        let module = custom.import?;
        NamedTypeImport {
            specifier: module.0.to_owned(),
            imported: custom.name.to_owned(),
            local: custom_scalar_local_name(scalar.dereference()),
        }
        .wrap_some()
    })
}

/// `imports` in specifier order.
pub(crate) fn sorted_imports(
    imports: impl Iterator<Item = NamedTypeImport>,
) -> impl Iterator<Item = TypeScriptItem> {
    imports
        .collect::<BTreeSet<_>>()
        .into_iter()
        .map(TypeScriptItem::NamedTypeImport)
}

fn required(key: &str, type_: TypeScriptType) -> PropertySignature {
    PropertySignature {
        key: key.to_owned(),
        optionality: Optionality::Required,
        type_,
        description: None,
    }
}

/// `Record<PropertyKey, never>`.
pub(crate) fn record_of_never() -> TypeScriptType {
    TypeScriptType::generic(
        "Record",
        vec![
            TypeScriptType::reference("PropertyKey"),
            TypeScriptType::Keyword(TypeScriptKeyword::Never),
        ],
    )
}

/// The specifier that imports the source file `source` from the artifact at `from`: `source`
/// relative to `from`'s folder inside the artifact folder, `/`-separated, `./`-prefixed, the
/// extension dropped. `None` when no relative path leads there.
fn source_specifier(
    from: &ArtifactPath,
    source: RelativePathToSourceFile,
    options: &Options,
) -> Option<String> {
    let from_folder = options
        .artifact_folder
        .join(Path::new(from.0.as_str()).parent()?);
    let relative =
        pathdiff::diff_paths(Path::new(source.lookup()).with_extension(""), from_folder)?;
    let segments = relative
        .components()
        .map(|component| match component {
            Component::ParentDir => "..".wrap_some(),
            Component::Normal(segment) => segment.to_str(),
            Component::CurDir | Component::RootDir | Component::Prefix(_) => None,
        })
        .collect::<Option<Vec<&str>>>()?;
    std::iter::once(".")
        .chain(segments)
        .collect::<Vec<_>>()
        .join("/")
        .wrap_some()
}
