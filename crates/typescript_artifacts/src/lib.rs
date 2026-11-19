//! TypeScript artifacts: the values generators build, and their printing as TypeScript.

mod custom_scalar;
mod entrypoint;
mod generate_types;
mod imports;
mod iso_overloads;
mod module;
mod swc;

use std::collections::BTreeSet;

use common_lang_types::{ConstExportName, EntityNameAndSelectableName};
use isograph_compiler::{ArtifactPath, Generation, GenerationOptions, ProjectArtifacts};
use prelude::Postfix;

pub use custom_scalar::{ModuleSpecifier, TypeScriptCustomScalarType};
pub use module::{ArtifactImport, ArtifactModule, ArtifactObjectEntry, ArtifactValue};

use crate::entrypoint::{NORMALIZATION_AST, REFETCH_READER, RESOLVER_READER, stub_artifact};
use crate::module::{TypeScriptModule, module_of_artifact};

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum TypeScriptGenerationError {
    #[error(
        "`{}.{}` is not exported with `export const`, so its type cannot import its resolver.",
        .0.parent_entity_name,
        .0.selectable_name
    )]
    MissingExport(EntityNameAndSelectableName),
    #[error(
        "No relative path leads from the artifact folder to the source file of `{}.{}`.",
        .0.parent_entity_name,
        .0.selectable_name
    )]
    SourceNotRelative(EntityNameAndSelectableName),
}

const TSCONFIG_JSON: &str = "{\n    \"compilerOptions\": {\n        \"noUnusedLocals\": false,\n        \"noUnusedParameters\": false,\n        \"jsx\": \"preserve\",\n        \"esModuleInterop\": true\n    }\n}\n";

/// The path and text of every artifact of `project`, in this order: per field its param type,
/// parameters type, output type, resolver reader, and refetch reader; per entrypoint and
/// loadably selected field its entrypoint and normalization AST; per link type its output type;
/// `iso.ts`; `tsconfig.json`. `export_name` reads a literal's `export const` name.
pub fn generate_artifacts<TLiteralContext>(
    project: &ProjectArtifacts<TLiteralContext>,
    options: &GenerationOptions<TypeScriptCustomScalarType>,
    export_name: impl Fn(&TLiteralContext) -> Option<ConstExportName>,
) -> Result<Vec<(ArtifactPath, String)>, TypeScriptGenerationError> {
    let generated = |generation: Generation| matches!(generation, Generation::Generated);
    let per_field = project
        .fields
        .iter()
        .map(|field| {
            let output_type = if generated(field.output_type) {
                generate_types::output_type_file(
                    field.field,
                    field.resolver.path,
                    export_name(field.resolver.context.reference()),
                    options,
                )?
                .wrap_some()
            } else {
                None
            };
            let modules = std::iter::once(generate_types::param_type_file(field, options))
                .chain(field.parameters.as_ref().map(|parameters| {
                    generate_types::parameters_type_file(field.field, parameters, options)
                }))
                .chain(output_type)
                .map(|module| printed(module.reference()))
                .chain(
                    generated(field.resolver_reader)
                        .then(|| printed_artifact(stub_artifact(field.field, RESOLVER_READER))),
                )
                .chain(
                    generated(field.loadable_artifacts)
                        .then(|| printed_artifact(stub_artifact(field.field, REFETCH_READER))),
                )
                .collect::<Vec<_>>();
            modules.wrap_ok()
        })
        .collect::<Result<Vec<_>, TypeScriptGenerationError>>()?;
    let entrypoints: BTreeSet<EntityNameAndSelectableName> = project
        .entrypoints
        .iter()
        .copied()
        .chain(
            project
                .fields
                .iter()
                .filter(|field| generated(field.loadable_artifacts))
                .map(|field| field.field),
        )
        .collect();
    let per_entrypoint = entrypoints.into_iter().flat_map(|entrypoint| {
        [
            printed_artifact(entrypoint::entrypoint_artifact(entrypoint)),
            printed_artifact(stub_artifact(entrypoint, NORMALIZATION_AST)),
        ]
    });
    let link_types = project
        .link_types
        .iter()
        .map(|link| printed(generate_types::link_type_file(link).reference()));
    per_field
        .into_iter()
        .flatten()
        .chain(per_entrypoint)
        .chain(link_types)
        .chain(std::iter::once(printed(
            iso_overloads::iso_overload_module(project).reference(),
        )))
        .chain(std::iter::once((
            ArtifactPath("tsconfig.json".to_owned()),
            TSCONFIG_JSON.to_owned(),
        )))
        .collect::<Vec<_>>()
        .wrap_ok()
}

fn printed(module: &TypeScriptModule) -> (ArtifactPath, String) {
    (module.path.clone(), swc::print(module))
}

fn printed_artifact(artifact: ArtifactModule) -> (ArtifactPath, String) {
    printed(module_of_artifact(artifact.reference()).reference())
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};
    use std::path::PathBuf;

    use common_lang_types::{
        ConstExportName, DescriptionValue, EntityName, EntityNameAndSelectableName, SelectionName,
    };
    use intern::string_key::Intern;
    use isograph_compiler::{
        ArtifactInnerType, ArtifactInputField, ArtifactInputType, ArtifactObjectField,
        ArtifactObjectType, ArtifactParameter, ArtifactPath, ClientFieldKind, FieldArtifactKind,
        FieldArtifacts, FieldDeclarationSource, Generation, GenerationOptions, LinkType,
        ParametersType, ProjectArtifacts, ScalarKind, ScalarValueKind, UnionWrapper, Wrapper,
        WrapperVariant,
    };
    use nonempty::NonEmpty;
    use prelude::Postfix;

    use super::{
        ModuleSpecifier, TypeScriptCustomScalarType, TypeScriptGenerationError, generate_artifacts,
    };

    type Context = Option<ConstExportName>;

    fn entity(name: &str) -> EntityName {
        name.intern().to()
    }

    fn named(parent: &str, selectable: &str) -> EntityNameAndSelectableName {
        EntityNameAndSelectableName {
            parent_entity_name: entity(parent),
            selectable_name: selectable.intern().to(),
        }
    }

    fn key(name: &str) -> SelectionName {
        name.intern().to()
    }

    fn string() -> ScalarKind {
        ScalarKind::Known(ScalarValueKind::String)
    }

    fn nullable<T>(variant: WrapperVariant<T>) -> Wrapper<T> {
        Wrapper::Union(UnionWrapper(vec![variant, WrapperVariant::Null]))
    }

    fn data_field(name: &str, type_: Wrapper<ArtifactInnerType>) -> ArtifactObjectField {
        ArtifactObjectField {
            key: key(name),
            description: None,
            type_,
        }
    }

    fn scalar_field(name: &str, scalar: ScalarKind) -> ArtifactObjectField {
        data_field(name, Wrapper::Inner(ArtifactInnerType::Scalar(scalar)))
    }

    /// A plain client field `parent.name` exported as `name` from `src/<parent>.ts`, with
    /// `data`, no parameters, and only its param type generated.
    fn field(parent: &str, name: &str, data: Vec<ArtifactObjectField>) -> FieldArtifacts<Context> {
        FieldArtifacts {
            field: named(parent, name),
            kind: FieldArtifactKind::ClientField(ClientFieldKind::Plain),
            resolver: FieldDeclarationSource {
                path: format!("src/{parent}.ts").intern().to(),
                context: name.intern().to::<ConstExportName>().wrap_some(),
            },
            data: ArtifactObjectType(data),
            parameters: None,
            output_type: Generation::NotGenerated,
            resolver_reader: Generation::NotGenerated,
            loadable_artifacts: Generation::NotGenerated,
        }
    }

    fn project(fields: Vec<FieldArtifacts<Context>>) -> ProjectArtifacts<Context> {
        ProjectArtifacts {
            fields,
            entities: Vec::new(),
            entrypoints: Vec::new(),
            link_types: Vec::new(),
        }
    }

    fn options(
        custom_scalar_types: BTreeMap<EntityName, TypeScriptCustomScalarType>,
    ) -> GenerationOptions<TypeScriptCustomScalarType> {
        GenerationOptions {
            artifact_folder: PathBuf::from("src/__isograph"),
            custom_scalar_types,
        }
    }

    fn generated_with(
        project: &ProjectArtifacts<Context>,
        custom_scalar_types: BTreeMap<EntityName, TypeScriptCustomScalarType>,
    ) -> Result<Vec<(ArtifactPath, String)>, TypeScriptGenerationError> {
        generate_artifacts(
            project,
            options(custom_scalar_types).reference(),
            |context| context.dereference(),
        )
    }

    fn generated(project: &ProjectArtifacts<Context>) -> Vec<(ArtifactPath, String)> {
        generated_with(project, BTreeMap::new()).expect("the fixture prints")
    }

    fn text_at<'a>(artifacts: &'a [(ArtifactPath, String)], path: &str) -> &'a str {
        artifacts
            .iter()
            .find(|(artifact_path, _)| artifact_path.0 == path)
            .map(|(_, text)| text.as_str())
            .expect("the test generated this artifact")
    }

    fn has_path(artifacts: &[(ArtifactPath, String)], path: &str) -> bool {
        artifacts
            .iter()
            .any(|(artifact_path, _)| artifact_path.0 == path)
    }

    /// The `iso` overload line whose matched string is `text`.
    fn overload_line<'a>(iso: &'a str, text: &str) -> &'a str {
        let needle = format!("MatchesWhitespaceAndString<\"{text}\", T>");
        iso.lines()
            .find(|line| line.contains(needle.as_str()))
            .expect("the test generated this overload")
    }

    fn position(iso: &str, text: &str) -> usize {
        iso.find(text).expect("the text is in the file")
    }

    #[test]
    fn scalars_lists_and_nullable_fields_print_their_types() {
        let artifacts = generated(
            project(vec![field(
                "Pet",
                "f",
                vec![
                    scalar_field("firstName", string()),
                    data_field(
                        "nickname",
                        nullable(WrapperVariant::Inner(ArtifactInnerType::Scalar(string()))),
                    ),
                    data_field(
                        "friends",
                        Wrapper::List(
                            Wrapper::Inner(ArtifactInnerType::Object(ArtifactObjectType(
                                scalar_field("id", string()).wrap_vec(),
                            )))
                            .boxed(),
                        ),
                    ),
                    scalar_field("age", ScalarKind::Known(ScalarValueKind::Number)),
                    scalar_field("good", ScalarKind::Known(ScalarValueKind::Boolean)),
                    scalar_field("picture", ScalarKind::Custom(entity("Url"))),
                ],
            )])
            .reference(),
        );
        let text = text_at(artifacts.as_slice(), "Pet/f/param_type.ts");
        assert!(text.contains("readonly firstName: string;"), "{text}");
        assert!(
            text.contains("readonly nickname: (string | null);"),
            "{text}"
        );
        assert!(text.contains("readonly friends: ReadonlyArray<{"), "{text}");
        assert!(text.contains("readonly id: string;"), "{text}");
        assert!(text.contains("readonly age: number;"), "{text}");
        assert!(text.contains("readonly good: boolean;"), "{text}");
        assert!(text.contains("readonly picture: unknown;"), "{text}");
        assert!(
            text.contains("readonly parameters: Record<PropertyKey, never>;"),
            "{text}"
        );
        assert!(!text.contains("import"), "{text}");
        assert!(!has_path(artifacts.as_slice(), "Pet/f/parameters_type.ts"));
    }

    #[test]
    fn a_custom_scalar_prints_the_profiles_type_and_imports_it_once() {
        let mut with_parameters = field(
            "Pet",
            "f",
            vec![
                scalar_field("price", ScalarKind::Custom(entity("Price"))),
                scalar_field("other", ScalarKind::Custom(entity("Price"))),
                scalar_field("url", ScalarKind::Custom(entity("Url"))),
            ],
        );
        with_parameters.parameters = ParametersType {
            parameters: NonEmpty::new(ArtifactParameter {
                name: "p".intern().to(),
                type_: Wrapper::Inner(ArtifactInputType::Scalar(ScalarKind::Custom(entity(
                    "Price",
                )))),
            }),
            input_objects: BTreeMap::new(),
        }
        .wrap_some();
        let artifacts = generated_with(
            project(with_parameters.wrap_vec()).reference(),
            BTreeMap::from([
                (
                    entity("Url"),
                    TypeScriptCustomScalarType {
                        name: "string",
                        import: None,
                    },
                ),
                (
                    entity("Price"),
                    TypeScriptCustomScalarType {
                        name: "Decimal",
                        import: ModuleSpecifier("decimal.js").wrap_some(),
                    },
                ),
            ]),
        )
        .expect("the fixture prints");
        let import = "import type { Decimal as Price__custom_scalar } from \"decimal.js\";";
        let param = text_at(artifacts.as_slice(), "Pet/f/param_type.ts");
        assert!(
            param.contains("readonly price: Price__custom_scalar;"),
            "{param}"
        );
        assert!(param.contains("readonly url: string;"), "{param}");
        assert_eq!(param.matches(import).count(), 1, "{param}");
        assert_eq!(param.matches("import").count(), 2, "{param}");
        let parameters = text_at(artifacts.as_slice(), "Pet/f/parameters_type.ts");
        assert_eq!(parameters.matches(import).count(), 1, "{parameters}");
    }

    #[test]
    fn a_selected_client_field_is_its_output_type_imported_once() {
        let output = || {
            data_field(
                "fullName",
                Wrapper::Inner(ArtifactInnerType::ClientFieldOutput(named(
                    "Pet", "fullName",
                ))),
            )
        };
        let artifacts = generated(
            project(vec![field(
                "Query",
                "Home",
                vec![
                    output(),
                    data_field(
                        "pet",
                        Wrapper::Inner(ArtifactInnerType::Object(ArtifactObjectType(
                            output().wrap_vec(),
                        ))),
                    ),
                ],
            )])
            .reference(),
        );
        let text = text_at(artifacts.as_slice(), "Query/Home/param_type.ts");
        let import =
            "import type { Pet__fullName__output_type } from \"./../../Pet/fullName/output_type\";";
        assert_eq!(text.matches(import).count(), 1, "{text}");
        assert!(
            text.contains("readonly fullName: Pet__fullName__output_type;"),
            "{text}"
        );
    }

    #[test]
    fn a_description_is_a_jsdoc_comment_before_its_property() {
        let described = |name: &str, description: &str| ArtifactObjectField {
            description: description.intern().to::<DescriptionValue>().wrap_some(),
            ..scalar_field(name, string())
        };
        let artifacts = generated(
            project(vec![field(
                "Pet",
                "f",
                vec![
                    described("id", "The pet's ID"),
                    described("x", "ends */ here"),
                    scalar_field("y", string()),
                ],
            )])
            .reference(),
        );
        let text = text_at(artifacts.as_slice(), "Pet/f/param_type.ts");
        assert!(
            text.contains("/**\nThe pet's ID\n*/ readonly id: string;"),
            "{text}"
        );
        assert!(
            text.contains("/**\nends *\\/ here\n*/ readonly x: string;"),
            "{text}"
        );
        let y_line = text
            .lines()
            .find(|line| line.contains("readonly y: string;"))
            .expect("the fixture selects y");
        assert_eq!(y_line.trim_start(), "readonly y: string;", "{text}");
    }

    #[test]
    fn variables_print_a_parameters_type_with_optional_nullable_parameters() {
        let mut with_parameters = field("Pet", "f", Vec::new());
        with_parameters.parameters = ParametersType {
            parameters: NonEmpty::from((
                ArtifactParameter {
                    name: "id".intern().to(),
                    type_: Wrapper::Inner(ArtifactInputType::Scalar(string())),
                },
                vec![
                    ArtifactParameter {
                        name: "limit".intern().to(),
                        type_: nullable(WrapperVariant::Inner(ArtifactInputType::Scalar(
                            ScalarKind::Known(ScalarValueKind::Number),
                        ))),
                    },
                    ArtifactParameter {
                        name: "filter".intern().to(),
                        type_: Wrapper::Inner(ArtifactInputType::InputObject(entity("Filter"))),
                    },
                ],
            )),
            input_objects: BTreeMap::from([(
                entity("Filter"),
                ArtifactInputField {
                    name: "and".intern().to(),
                    type_: nullable(WrapperVariant::List(
                        Wrapper::Inner(ArtifactInputType::InputObject(entity("Filter"))).boxed(),
                    )),
                }
                .wrap_vec(),
            )]),
        }
        .wrap_some();
        let artifacts = generated(project(with_parameters.wrap_vec()).reference());
        let parameters = text_at(artifacts.as_slice(), "Pet/f/parameters_type.ts");
        assert!(parameters.contains("readonly id: string;"), "{parameters}");
        assert!(
            parameters.contains("readonly limit?: number | null | void;"),
            "{parameters}"
        );
        assert!(
            parameters.contains("readonly filter: Filter;"),
            "{parameters}"
        );
        assert!(parameters.contains("type Filter = {"), "{parameters}");
        assert!(
            parameters.contains("readonly and?: ReadonlyArray<Filter> | null | void;"),
            "{parameters}"
        );
        let param = text_at(artifacts.as_slice(), "Pet/f/param_type.ts");
        assert!(
            param.contains("import type { Pet__f__parameters } from \"./parameters_type\";"),
            "{param}"
        );
        assert!(
            param.contains("readonly parameters: Pet__f__parameters;"),
            "{param}"
        );
    }

    #[test]
    fn an_output_type_imports_its_resolvers_type_from_the_source_file() {
        let mut full_name = field("Pet", "fullName", Vec::new());
        full_name.output_type = Generation::Generated;
        let artifacts = generated(project(full_name.clone().wrap_vec()).reference());
        let text = text_at(artifacts.as_slice(), "Pet/fullName/output_type.ts");
        assert!(
            text.contains("import type { fullName as resolver } from \"./../../../Pet\";"),
            "{text}"
        );
        assert!(
            text.contains("export type Pet__fullName__output_type = ReturnType<typeof resolver>;"),
            "{text}"
        );
        full_name.resolver.context = None;
        assert_eq!(
            generated_with(project(full_name.wrap_vec()).reference(), BTreeMap::new()),
            TypeScriptGenerationError::MissingExport(named("Pet", "fullName")).wrap_err()
        );
    }

    #[test]
    fn readers_refetch_readers_and_normalization_asts_are_empty_objects() {
        let mut checkins = field("Pet", "Checkins", Vec::new());
        checkins.resolver_reader = Generation::Generated;
        checkins.loadable_artifacts = Generation::Generated;
        let artifacts = generated(project(checkins.wrap_vec()).reference());
        let texts: Vec<&str> = [
            "resolver_reader.ts",
            "refetch_reader.ts",
            "normalization_ast.ts",
        ]
        .into_iter()
        .map(|file| {
            text_at(
                artifacts.as_slice(),
                format!("Pet/Checkins/{file}").as_str(),
            )
        })
        .collect();
        assert_eq!(texts, ["export default {};\n"; 3]);
        assert!(has_path(artifacts.as_slice(), "Pet/Checkins/entrypoint.ts"));
    }

    #[test]
    fn an_entrypoint_imports_its_normalization_ast_and_reader() {
        let mut home = field("Query", "HomeRoute", Vec::new());
        home.resolver_reader = Generation::Generated;
        let artifacts = generated(
            ProjectArtifacts {
                entrypoints: named("Query", "HomeRoute").wrap_vec(),
                ..project(home.wrap_vec())
            }
            .reference(),
        );
        let text = text_at(artifacts.as_slice(), "Query/HomeRoute/entrypoint.ts");
        assert!(text.contains("kind: \"Entrypoint\""), "{text}");
        assert!(text.contains("name: \"Query.HomeRoute\""), "{text}");
        assert!(
            text.contains("normalizationAst: Query__HomeRoute__normalization_ast"),
            "{text}"
        );
        assert!(
            text.contains("readerArtifact: Query__HomeRoute__resolver_reader"),
            "{text}"
        );
        assert!(
            text.contains(
                "import Query__HomeRoute__normalization_ast from \"./normalization_ast\";"
            ),
            "{text}"
        );
        assert!(
            text.contains("import Query__HomeRoute__resolver_reader from \"./resolver_reader\";"),
            "{text}"
        );
    }

    #[test]
    fn a_link_type_is_one_object_per_concrete_type() {
        let link = |name: &str, concrete: &[&str]| LinkType {
            entity: entity(name),
            concrete_types: concrete.iter().map(|concrete| entity(concrete)).collect(),
        };
        let artifacts = generated(
            ProjectArtifacts {
                link_types: vec![
                    link("Pet", &["Pet"]),
                    link("Node", &["A", "B"]),
                    link("Empty", &[]),
                ],
                ..project(Vec::new())
            }
            .reference(),
        );
        let pet = text_at(artifacts.as_slice(), "Pet/__link/output_type.ts");
        assert!(
            pet.contains("export type Pet____link__output_type = {"),
            "{pet}"
        );
        assert!(pet.contains("readonly __link: string;"), "{pet}");
        assert!(pet.contains("readonly __typename: \"Pet\";"), "{pet}");
        let node = text_at(artifacts.as_slice(), "Node/__link/output_type.ts");
        assert!(node.contains("readonly __typename: \"A\";"), "{node}");
        assert!(node.contains("} | {"), "{node}");
        assert!(node.contains("readonly __typename: \"B\";"), "{node}");
        assert_eq!(
            text_at(artifacts.as_slice(), "Empty/__link/output_type.ts"),
            "export type Empty____link__output_type = never;\n"
        );
    }

    #[test]
    fn iso_overloads_are_typed_by_field_kind() {
        let mut home = field("Query", "HomeRoute", Vec::new());
        home.kind = FieldArtifactKind::ClientField(ClientFieldKind::Component);
        let mut best_friend = field("Pet", "bestFriend", Vec::new());
        best_friend.kind =
            FieldArtifactKind::Pointer(nullable(WrapperVariant::Inner(entity("Pet"))));
        let mut owner = field("Pet", "owner", Vec::new());
        owner.kind = FieldArtifactKind::Pointer(Wrapper::Inner(entity("Owner")));
        let artifacts = generated(
            ProjectArtifacts {
                fields: vec![
                    best_friend,
                    field("Pet", "fullName", Vec::new()),
                    owner,
                    home,
                ],
                entities: entity("Owner").wrap_vec(),
                entrypoints: named("Query", "HomeRoute").wrap_vec(),
                link_types: vec![
                    LinkType {
                        entity: entity("Owner"),
                        concrete_types: entity("Owner").wrap_vec(),
                    },
                    LinkType {
                        entity: entity("Pet"),
                        concrete_types: entity("Pet").wrap_vec(),
                    },
                ],
            }
            .reference(),
        );
        let iso = text_at(artifacts.as_slice(), "iso.ts");
        assert!(
            overload_line(iso, "field Pet.fullName")
                .ends_with("): IdentityWithParam<Pet__fullName__param>;"),
            "{iso}"
        );
        assert!(
            overload_line(iso, "field Query.HomeRoute")
                .ends_with("): IdentityWithParamComponent<Query__HomeRoute__param>;"),
            "{iso}"
        );
        assert!(
            overload_line(iso, "field Pet.bestFriend").ends_with(
                "): IdentityWithParam<Pet__bestFriend__param, (Pet____link__output_type | null)>;"
            ),
            "{iso}"
        );
        assert!(
            overload_line(iso, "field Pet.owner")
                .ends_with("): IdentityWithParam<Pet__owner__param, Owner____link__output_type>;"),
            "{iso}"
        );
        assert!(
            overload_line(iso, "entity Owner").ends_with("): void;"),
            "{iso}"
        );
        assert!(
            overload_line(iso, "entrypoint Query.HomeRoute")
                .ends_with("): typeof Query__HomeRoute__entrypoint;"),
            "{iso}"
        );
        let param_import = position(
            iso,
            "import type { Query__HomeRoute__param } from \"./Query/HomeRoute/param_type\";",
        );
        let link_import = position(
            iso,
            "import type { Pet____link__output_type } from \"./Pet/__link/output_type\";",
        );
        let entrypoint_import = position(
            iso,
            "import type Query__HomeRoute__entrypoint from \"./Query/HomeRoute/entrypoint\";",
        );
        assert!(
            param_import < link_import && link_import < entrypoint_import,
            "{iso}"
        );
        assert!(iso.lines().all(|line| line.trim_end() == line), "{iso}");
    }

    #[test]
    fn a_longer_field_name_comes_before_its_prefix() {
        let artifacts = generated(
            project(vec![
                field("Pet", "Avatar", Vec::new()),
                field("Pet", "AvatarCard", Vec::new()),
            ])
            .reference(),
        );
        let iso = text_at(artifacts.as_slice(), "iso.ts");
        assert!(
            position(iso, "<\"field Pet.AvatarCard\", T>")
                < position(iso, "<\"field Pet.Avatar\", T>")
        );
    }

    #[test]
    fn tsconfig_json_is_upstreams_text_with_a_trailing_newline() {
        let artifacts = generated(project(Vec::new()).reference());
        assert_eq!(
            text_at(artifacts.as_slice(), "tsconfig.json"),
            "{\n    \"compilerOptions\": {\n        \"noUnusedLocals\": false,\n        \"noUnusedParameters\": false,\n        \"jsx\": \"preserve\",\n        \"esModuleInterop\": true\n    }\n}\n"
        );
    }

    #[test]
    fn no_path_is_generated_twice() {
        let mut checkins = field("Pet", "Checkins", Vec::new());
        checkins.resolver_reader = Generation::Generated;
        checkins.loadable_artifacts = Generation::Generated;
        checkins.output_type = Generation::Generated;
        let artifacts = generated(
            ProjectArtifacts {
                entrypoints: named("Pet", "Checkins").wrap_vec(),
                ..project(checkins.wrap_vec())
            }
            .reference(),
        );
        let paths: BTreeSet<&str> = artifacts.iter().map(|(path, _)| path.0.as_str()).collect();
        assert_eq!(paths.len(), artifacts.len());
    }
}
