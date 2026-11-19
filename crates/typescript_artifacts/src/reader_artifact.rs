use common_lang_types::ConstExportName;
use isograph_compiler::{FieldArtifacts, ReaderSelection, ReaderSelectionSet};
use prelude::Postfix;

use crate::TypeScriptGenerationError;
use crate::entrypoint::{RESOLVER_READER, RUNTIME_PACKAGE, field_artifact_path};
use crate::generate_types::{
    OUTPUT_TYPE, Options, PARAM_TYPE, entity_name_type, output_type_name, param_type_name,
    sorted_imports, source_specifier,
};
use crate::imports::{imports_of, named_imports_of, relative_artifact_specifier};
use crate::module::{
    ArtifactImport, ArtifactValue, DeclaredConst, NamedImport, TypeScriptItem, TypeScriptModule,
    TypeScriptType,
};
use crate::normalization_ast::{arguments_value, condition_value, entry, kind, string};

const READER_ARTIFACT_TYPE: &str = "ReaderArtifact";
const BINDING: &str = "readerArtifact";

/// `P/f/resolver_reader.ts`: `reader` declared as a `ReaderArtifact` of the field's param and
/// output types, with the resolver exported as `export` from the field's source file.
pub(crate) fn reader_artifact_file<TLiteralContext>(
    field: &FieldArtifacts<TLiteralContext>,
    reader: &ReaderSelectionSet,
    export: Option<ConstExportName>,
    options: &Options,
) -> Result<TypeScriptModule, TypeScriptGenerationError> {
    let path = field_artifact_path(field.field, RESOLVER_READER);
    let export = export.ok_or(TypeScriptGenerationError::MissingExport(field.field))?;
    let specifier = source_specifier(path.reference(), field.resolver.path, options)
        .ok_or(TypeScriptGenerationError::SourceNotRelative(field.field))?;
    let value = ArtifactValue::Object(vec![
        entry("readerAst", selection_set_value(reader)),
        entry(
            "resolver",
            ArtifactValue::NamedImport(NamedImport {
                specifier,
                imported: export.to_string(),
                local: "resolver".to_owned(),
            }),
        ),
    ]);
    let type_import = |file: &str, name: String| NamedImport {
        specifier: relative_artifact_specifier(
            path.reference(),
            field_artifact_path(field.field, file).reference(),
        ),
        imported: name.clone(),
        local: name,
    };
    let param = param_type_name(field.field);
    let output = output_type_name(field.field);
    let value_imports: Vec<TypeScriptItem> = named_imports_of(value.reference())
        .into_iter()
        .cloned()
        .map(TypeScriptItem::NamedImport)
        .chain(
            imports_of(value.reference())
                .into_iter()
                .cloned()
                .map(TypeScriptItem::DefaultImport),
        )
        .collect();
    let items = std::iter::once(TypeScriptItem::NamedTypeImport(NamedImport {
        specifier: RUNTIME_PACKAGE.to_owned(),
        imported: READER_ARTIFACT_TYPE.to_owned(),
        local: READER_ARTIFACT_TYPE.to_owned(),
    }))
    .chain(sorted_imports(
        [
            type_import(OUTPUT_TYPE, output.clone()),
            type_import(PARAM_TYPE, param.clone()),
        ]
        .into_iter(),
    ))
    .chain(value_imports)
    .chain([
        TypeScriptItem::DeclaredConst(DeclaredConst {
            binding: BINDING.to_owned(),
            type_: TypeScriptType::generic(
                READER_ARTIFACT_TYPE,
                vec![
                    TypeScriptType::reference(param.as_str()),
                    TypeScriptType::reference(output.as_str()),
                    entity_name_type(field.concrete_entities.as_slice()),
                ],
            ),
            value,
        }),
        TypeScriptItem::ExportDefaultBinding(BINDING.to_owned()),
    ])
    .collect();
    TypeScriptModule { path, items }.wrap_ok()
}

fn selection_set_value(selection_set: &ReaderSelectionSet) -> ArtifactValue {
    ArtifactValue::Array(selection_set.0.iter().map(selection_value).collect())
}

fn selection_value(selection: &ReaderSelection) -> ArtifactValue {
    match selection {
        ReaderSelection::Scalar(scalar) => ArtifactValue::Object(vec![
            kind("Scalar"),
            entry("alias", string(scalar.alias)),
            entry("selectableName", string(scalar.selectable)),
            entry("arguments", arguments_value(scalar.arguments.reference())),
        ]),
        ReaderSelection::Linked(linked) => ArtifactValue::Object(vec![
            kind("Linked"),
            entry("alias", string(linked.alias)),
            entry("selectableName", string(linked.selectable)),
            entry("arguments", arguments_value(linked.arguments.reference())),
            entry(
                "selections",
                selection_set_value(linked.selections.reference()),
            ),
        ]),
        ReaderSelection::Refinement(refinement) => ArtifactValue::Object(vec![
            kind("Refinement"),
            entry("alias", string(refinement.alias)),
            entry(
                "condition",
                condition_value(refinement.condition.reference()),
            ),
            entry(
                "selections",
                selection_set_value(refinement.selections.reference()),
            ),
        ]),
        ReaderSelection::Resolver(resolver) => ArtifactValue::Object(vec![
            kind("Resolver"),
            entry("alias", string(resolver.alias)),
            entry("arguments", arguments_value(resolver.arguments.reference())),
            entry(
                "readerArtifact",
                ArtifactValue::Import(ArtifactImport(field_artifact_path(
                    resolver.field,
                    RESOLVER_READER,
                ))),
            ),
        ]),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    use common_lang_types::{ConstExportName, EntityName, EntityNameAndSelectableName};
    use intern::string_key::Intern;
    use isograph_compiler::{
        ArtifactObjectType, Condition, FieldArtifactKind, FieldArtifacts, FieldDeclarationSource,
        Generation, GenerationOptions, NormalizationArguments, ReaderLinkedSelection,
        ReaderRefinementSelection, ReaderResolverSelection, ReaderScalarSelection, ReaderSelection,
        ReaderSelectionSet,
    };
    use prelude::Postfix;

    use super::reader_artifact_file;
    use crate::TypeScriptGenerationError;
    use crate::generate_types::{Options, param_type_file};
    use crate::swc::print;

    fn entity(name: &str) -> EntityName {
        name.intern().to()
    }

    fn named(parent: &str, selectable: &str) -> EntityNameAndSelectableName {
        EntityNameAndSelectableName {
            parent_entity_name: entity(parent),
            selectable_name: selectable.intern().to(),
        }
    }

    fn options() -> Options {
        GenerationOptions {
            artifact_folder: PathBuf::from("src/__isograph"),
            custom_scalar_types: BTreeMap::new(),
        }
    }

    /// `parent.name`, declared in `source` and reachable, with no data.
    fn field_in(parent: &str, name: &str, source: &str) -> FieldArtifacts<()> {
        FieldArtifacts {
            field: named(parent, name),
            kind: FieldArtifactKind::ClientField,
            resolver: FieldDeclarationSource {
                path: source.intern().to(),
                context: (),
            },
            data: ArtifactObjectType(Vec::new()),
            parameters: None,
            output_type: Generation::Generated,
            resolver_reader: ReaderSelectionSet(Vec::new()).wrap_some(),
            concrete_entities: entity(parent).wrap_vec(),
        }
    }

    fn full_name() -> FieldArtifacts<()> {
        field_in("Pet", "fullName", "src/Pet.ts")
    }

    fn export(name: &str) -> Option<ConstExportName> {
        name.intern().to::<ConstExportName>().wrap_some()
    }

    fn printed(
        field: &FieldArtifacts<()>,
        reader: Vec<ReaderSelection>,
    ) -> Result<String, TypeScriptGenerationError> {
        reader_artifact_file(
            field,
            ReaderSelectionSet(reader).reference(),
            export(field.field.selectable_name.to_string().as_str()),
            options().reference(),
        )
        .map(|module| print(module.reference()))
    }

    fn scalar(alias: &str, selectable: &str) -> ReaderSelection {
        ReaderSelection::Scalar(ReaderScalarSelection {
            alias: alias.intern().to(),
            selectable: selectable.intern().to(),
            arguments: NormalizationArguments::default(),
        })
    }

    fn full_name_resolver(alias: &str) -> ReaderSelection {
        ReaderSelection::Resolver(ReaderResolverSelection {
            alias: alias.intern().to(),
            field: named("Pet", "fullName"),
            arguments: NormalizationArguments::default(),
        })
    }

    #[test]
    fn the_module_imports_its_types_and_resolver_then_declares_and_exports_the_artifact() {
        let text = printed(
            full_name().reference(),
            vec![
                scalar("firstName", "firstName"),
                scalar("lastName", "lastName"),
            ],
        )
        .expect("the fixture prints");
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(
            lines[..4],
            [
                "import type { ReaderArtifact } from \"@isograph/runtime\";",
                "import type { Pet__fullName__output_type } from \"./output_type\";",
                "import type { Pet__fullName__param } from \"./param_type\";",
                "import { fullName as resolver } from \"./../../../Pet\";",
            ],
            "{text}"
        );
        assert!(
            lines.iter().any(|line| line.starts_with(
                "const readerArtifact: ReaderArtifact<Pet__fullName__param, Pet__fullName__output_type, \"Pet\"> = {"
            )),
            "{text}"
        );
        assert_eq!(lines.last(), Some(&"export default readerArtifact;"));
        assert!(text.contains("resolver: resolver"), "{text}");
        assert!(!text.contains("discriminator"), "{text}");
    }

    #[test]
    fn the_third_type_argument_is_the_union_of_the_concrete_entities() {
        let mut node = field_in("Node", "NodeLink", "src/NodeLink.ts");
        node.concrete_entities = ["BlogItem", "Checkin", "Pet"]
            .into_iter()
            .map(entity)
            .collect();
        let text = printed(node.reference(), Vec::new()).expect("the fixture prints");
        assert!(
            text.contains("Node__NodeLink__output_type, \"BlogItem\" | \"Checkin\" | \"Pet\">"),
            "{text}"
        );
        node.concrete_entities = Vec::new();
        let text = printed(node.reference(), Vec::new()).expect("the fixture prints");
        assert!(
            text.contains("Node__NodeLink__output_type, never>"),
            "{text}"
        );
    }

    #[test]
    fn the_param_type_ends_with_the_parent_entitys_link() {
        let pet =
            print(param_type_file(full_name().reference(), options().reference()).reference());
        assert!(
            pet.contains(
                "import type { Pet____link__output_type } from \"./../__link/output_type\";"
            ),
            "{pet}"
        );
        let parameters = pet
            .find("readonly parameters")
            .expect("the param type has parameters");
        let link = pet
            .find("readonly link: Pet____link__output_type;")
            .expect("the param type has link");
        assert!(parameters < link, "{pet}");
        let query = print(
            param_type_file(
                field_in("Query", "Home", "src/Home.ts").reference(),
                options().reference(),
            )
            .reference(),
        );
        assert!(
            query.contains(
                "import type { Query____link__output_type } from \"./../__link/output_type\";"
            ),
            "{query}"
        );
    }

    #[test]
    fn each_node_kind_prints_its_fields() {
        let text = printed(
            full_name().reference(),
            vec![
                scalar("friendName", "firstName"),
                ReaderSelection::Linked(ReaderLinkedSelection {
                    alias: "bestFriend".intern().to(),
                    selectable: "bestFriend".intern().to(),
                    arguments: NormalizationArguments::default(),
                    selections: ReaderSelectionSet(scalar("id", "id").wrap_vec()),
                }),
                ReaderSelection::Refinement(ReaderRefinementSelection {
                    alias: "asPet".intern().to(),
                    condition: Condition::SelectionEquals(entity("Pet")),
                    selections: ReaderSelectionSet(Vec::new()),
                }),
            ],
        )
        .expect("the fixture prints");
        for needle in [
            "kind: \"Scalar\"",
            "alias: \"friendName\"",
            "selectableName: \"firstName\"",
            "arguments: null",
            "kind: \"Linked\"",
            "selections: [",
            "kind: \"Refinement\"",
            "kind: \"SelectionEquals\"",
            "entityName: \"Pet\"",
        ] {
            assert!(text.contains(needle), "{needle} in {text}");
        }
    }

    #[test]
    fn two_resolver_nodes_of_one_field_import_its_reader_once() {
        let text = printed(
            field_in("Query", "Home", "src/Home.ts").reference(),
            vec![full_name_resolver("a"), full_name_resolver("b")],
        )
        .expect("the fixture prints");
        assert_eq!(
            text.matches("readerArtifact: Pet__fullName__resolver_reader")
                .count(),
            2,
            "{text}"
        );
        assert_eq!(
            text.matches(
                "import Pet__fullName__resolver_reader from \"./../../Pet/fullName/resolver_reader\";"
            )
            .count(),
            1,
            "{text}"
        );
    }

    #[test]
    fn a_missing_export_or_an_unreachable_source_is_an_error() {
        let field = full_name();
        assert_eq!(
            reader_artifact_file(
                field.reference(),
                ReaderSelectionSet(Vec::new()).reference(),
                None,
                options().reference(),
            ),
            TypeScriptGenerationError::MissingExport(field.field).wrap_err()
        );
        let outside = field_in("Pet", "fullName", "/elsewhere/Pet.ts");
        assert_eq!(
            reader_artifact_file(
                outside.reference(),
                ReaderSelectionSet(Vec::new()).reference(),
                export("fullName"),
                options().reference(),
            ),
            TypeScriptGenerationError::SourceNotRelative(outside.field).wrap_err()
        );
    }
}
