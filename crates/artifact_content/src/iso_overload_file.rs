use intern::Lookup;
use isograph_config::GenerateFileExtensionsOption;
use isograph_lang_types::{
    ClientScalarSelectableDirectiveSet, EmptyDirectiveSet, SelectionType, SelectionTypePostfix,
};
use pico::MemoRef;
use prelude::Postfix;
use std::{cmp::Ordering, collections::BTreeSet};

use common_lang_types::{
    ArtifactPath, ArtifactPathAndContent, EntityName, ExpectSelectableToExist, SelectableName,
};
use isograph_schema::{
    ClientScalarSelectable, CompilationProfile, EntrypointDeclarationInfo, IsographDatabase,
    LINK_FIELD_NAME, MemoRefClientSelectable, deprecated_client_scalar_selectable_named,
    deprecated_client_selectable_map, validated_entrypoints,
};

use crate::generate_artifacts::{ISO_TS_FILE_NAME, print_javascript_type_declaration};

fn build_iso_overload_for_entrypoint<TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
    client_scalar_selectable: MemoRef<ClientScalarSelectable<TCompilationProfile>>,
    file_extensions: GenerateFileExtensionsOption,
) -> (String, String) {
    let type_and_field = client_scalar_selectable
        .lookup(db)
        .entity_name_and_selectable_name();
    let formatted_field = format!(
        "entrypoint {}.{}",
        type_and_field.parent_entity_name, type_and_field.selectable_name
    );
    let mut s: String = "".to_string();
    let import = format!(
        "import entrypoint_{} from '../__isograph/{}/{}/entrypoint{}';\n",
        type_and_field.underscore_separated(),
        type_and_field.parent_entity_name,
        type_and_field.selectable_name,
        file_extensions.ts()
    );

    s.push_str(&format!(
        "
export function iso<T>(
  param: T & MatchesWhitespaceAndString<'{}', T>
): typeof entrypoint_{};\n",
        formatted_field,
        type_and_field.underscore_separated(),
    ));
    (import, s)
}

fn build_iso_overload_for_client_defined_type<TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
    client_type_and_variant: (
        MemoRefClientSelectable<TCompilationProfile>,
        ClientScalarSelectableDirectiveSet,
    ),
    file_extensions: GenerateFileExtensionsOption,
    link_types: &mut BTreeSet<EntityName>,
) -> (String, String) {
    let (client_type, variant) = client_type_and_variant;
    let type_and_field = match client_type_and_variant.0 {
        SelectionType::Scalar(s) => {
            let scalar = s.lookup(db);
            scalar.entity_name_and_selectable_name()
        }
        SelectionType::Object(o) => {
            let object = o.lookup(db);
            object.entity_name_and_selectable_name()
        }
    };

    let mut s: String = "".to_string();
    let import = format!(
        "import {{ type {}__param }} from './{}/{}/param_type{}';\n",
        type_and_field.underscore_separated(),
        type_and_field.parent_entity_name,
        type_and_field.selectable_name,
        file_extensions.ts()
    );

    let formatted_field = format!(
        "{} {}.{}",
        match client_type {
            SelectionType::Scalar(_) => "field",
            SelectionType::Object(_) => "pointer",
        },
        type_and_field.parent_entity_name,
        type_and_field.selectable_name
    );
    let client_type = match client_type {
        SelectionType::Scalar(s) => s.lookup(db).scalar_selected(),
        SelectionType::Object(o) => o.lookup(db).object_selected(),
    };
    if matches!(variant, ClientScalarSelectableDirectiveSet::Component(_)) {
        s.push_str(&format!(
            "
export function iso<T>(
  param: T & MatchesWhitespaceAndString<'{}', T>
): IdentityWithParamComponent<{}__param>;\n",
            formatted_field,
            type_and_field.underscore_separated(),
        ));
    } else if let SelectionType::Object(client_object_selectable) = client_type {
        link_types.insert(client_object_selectable.target_entity.inner().0);

        let link_field_name = *LINK_FIELD_NAME;
        let inner_text = format!(
            "{}__{link_field_name}__output_type",
            client_object_selectable.target_entity.inner()
        );

        s.push_str(&format!(
            "
export function iso<T>(
  param: T & MatchesWhitespaceAndString<'{}', T>
): IdentityWithParam<{}__param, {}>;\n",
            formatted_field,
            type_and_field.underscore_separated(),
            print_javascript_type_declaration(
                client_object_selectable.target_entity.reference(),
                inner_text
            )
        ));
    } else {
        s.push_str(&format!(
            "
export function iso<T>(
  param: T & MatchesWhitespaceAndString<'{}', T>
): IdentityWithParam<{}__param>;\n",
            formatted_field,
            type_and_field.underscore_separated(),
        ));
    }
    (import, s)
}

pub(crate) fn build_iso_overload_artifact<TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
    file_extensions: GenerateFileExtensionsOption,
    no_babel_transform: bool,
) -> ArtifactPathAndContent {
    let mut imports = "import type { IsographEntrypoint } from '@isograph/react';\n".to_string();
    let mut content = String::from(
        "
// This is the type given to regular client fields.
// This means that the type of the exported iso literal is exactly
// the type of the passed-in function, which takes one parameter
// of type TParam.
type IdentityWithParam<TParam extends object, TReturnConstraint = unknown> = <TClientFieldReturn extends TReturnConstraint>(
  clientField: (param: TParam) => TClientFieldReturn
) => (param: TParam) => TClientFieldReturn;

// This is the type given it to client fields with @component.
// This means that the type of the exported iso literal is exactly
// the type of the passed-in function, which takes two parameters.
// The first has type TParam, and the second has type TComponentProps.
//
// TComponentProps becomes the types of the props you must pass
// whenever the @component field is rendered.
type IdentityWithParamComponent<TParam extends object> = <
  TClientFieldReturn,
  TComponentProps = Record<PropertyKey, never>,
>(
  clientComponentField: (data: TParam, componentProps: TComponentProps) => TClientFieldReturn
) => (data: TParam, componentProps: TComponentProps) => TClientFieldReturn;

type WhitespaceCharacter = ' ' | '\\t' | '\\n';
type Whitespace<In> = In extends `${WhitespaceCharacter}${infer In}`
  ? Whitespace<In>
  : In;

// This is a recursive TypeScript type that matches strings that
// start with whitespace, followed by TString. So e.g. if we have
// ```
// export function iso<T>(
//   isographLiteralText: T & MatchesWhitespaceAndString<'field Query.foo', T>
// ): Bar;
// ```
// then, when you call
// ```
// const x = iso(`
//   field Query.foo ...
// `);
// ```
// then the type of `x` will be `Bar`, both in VSCode and when running
// tsc. This is how we achieve type safety — you can only use fields
// that you have explicitly selected.
type MatchesWhitespaceAndString<
  TString extends string,
  T
> = Whitespace<T> extends `${TString}${string}` ? T : never;\n",
    );

    let mut target_object_entity_names = BTreeSet::new();

    let client_defined_type_overloads =
        sorted_user_written_types(db)
            .into_iter()
            .map(|client_type| {
                build_iso_overload_for_client_defined_type(
                    db,
                    client_type,
                    file_extensions,
                    &mut target_object_entity_names,
                )
            });

    for (import, client_type_overload) in client_defined_type_overloads {
        imports.push_str(&import);
        content.push_str(&client_type_overload);
    }

    for target_object_entity_name in target_object_entity_names {
        let link_field_name = *LINK_FIELD_NAME;
        imports.push_str(&format!(
            "import {{ type {}__{link_field_name}__output_type }} from './{}/{link_field_name}/output_type{}';\n",
            target_object_entity_name,
            target_object_entity_name,
            file_extensions.ts()
        ));
    }

    let entrypoint_overloads = sorted_entrypoints(db)
        .into_iter()
        .map(|(field, _)| build_iso_overload_for_entrypoint(db, field, file_extensions));
    for (import, entrypoint_overload) in entrypoint_overloads {
        imports.push_str(&import);
        content.push_str(&entrypoint_overload);
    }

    if !no_babel_transform {
        content.push_str(
            "
export function iso(_isographLiteralText: string):
  | IdentityWithParam<any>
  | IdentityWithParamComponent<any>
  | IsographEntrypoint<any, any, any, any>
{\n",
        );
        content.push_str("  throw new Error('iso: Unexpected invocation at runtime. Either the Babel transform ' +
      'was not set up, or it failed to identify this call site. Make sure it ' +
      'is being used verbatim as `iso`. If you cannot use the babel transform, ' + 
      'set options.no_babel_transform to true in your Isograph config. ');\n}")
    } else {
        let switch_cases =
            sorted_entrypoints(db)
                .into_iter()
                .map(|(field, entrypoint_declaration_info)| {
                    let field = field.lookup(db);
                    format!(
                        "    case '{}':
      return entrypoint_{};\n",
                        entrypoint_declaration_info.iso_literal_text,
                        field
                            .entity_name_and_selectable_name()
                            .underscore_separated()
                    )
                });

        content.push_str(
            "
export function iso(isographLiteralText: string):
  | IdentityWithParam<any>
  | IdentityWithParamComponent<any>
  | IsographEntrypoint<any, any, any, any>
{
  switch (isographLiteralText) {\n",
        );

        for switch_case in switch_cases {
            content.push_str(&switch_case);
        }
        content.push_str(
            "  }
  return (clientFieldResolver: any) => clientFieldResolver;\n}",
        )
    }

    imports.push_str(&content);
    ArtifactPathAndContent {
        file_content: imports.into(),
        artifact_path: ArtifactPath {
            file_name: *ISO_TS_FILE_NAME,
            type_and_field: None,
        },
    }
}

fn sorted_user_written_types<TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
) -> Vec<(
    MemoRefClientSelectable<TCompilationProfile>,
    ClientScalarSelectableDirectiveSet,
)> {
    let mut client_types = deprecated_client_selectable_map(db)
        .as_ref()
        .expect("Expected client selectable map to be valid.")
        .iter()
        .flat_map(|(_, value)| {
            let value = value
                .as_ref()
                .expect("Expected client selectable to be valid");

            match value {
                SelectionType::Scalar(s) => match s.lookup(db).variant.reference() {
                    isograph_schema::ClientFieldVariant::UserWritten(_) => {}
                    isograph_schema::ClientFieldVariant::ImperativelyLoadedField(_) => return None,
                    isograph_schema::ClientFieldVariant::Link => return None,
                },
                SelectionType::Object(_) => {}
            };

            (*value).wrap_some()
        })
        .map(|selection_type| {
            let client_scalar_selection_directive_set = {
                match selection_type.reference() {
                    SelectionType::Scalar(scalar) => match scalar.lookup(db).variant.reference() {
                        isograph_schema::ClientFieldVariant::UserWritten(
                            user_written_client_type_info,
                        ) => user_written_client_type_info
                            .client_scalar_selectable_directive_set
                            .clone()
                            .expect(
                                "Expected directive set to have been validated. \
                                This is indicative of a bug in Isograph.",
                            ),
                        isograph_schema::ClientFieldVariant::ImperativelyLoadedField(_) => {
                            ClientScalarSelectableDirectiveSet::None(EmptyDirectiveSet {})
                        }
                        isograph_schema::ClientFieldVariant::Link => {
                            ClientScalarSelectableDirectiveSet::None(EmptyDirectiveSet {})
                        }
                    },
                    SelectionType::Object(_) => {
                        ClientScalarSelectableDirectiveSet::None(EmptyDirectiveSet {})
                    }
                }
            };
            (selection_type, client_scalar_selection_directive_set)
        })
        .collect::<Vec<_>>();

    client_types.sort_by(|client_type_1, client_type_2| {
        let (parent_1, selectable_name_1) = match client_type_1.0 {
            SelectionType::Scalar(s) => {
                let s = s.lookup(db);
                (s.parent_entity_name, s.name)
            }
            SelectionType::Object(o) => {
                let o = o.lookup(db);
                (o.parent_entity_name, o.name)
            }
        };
        let (parent_2, selectable_name_2) = match client_type_2.0 {
            SelectionType::Scalar(s) => {
                let s = s.lookup(db);
                (s.parent_entity_name, s.name)
            }
            SelectionType::Object(o) => {
                let o = o.lookup(db);
                (o.parent_entity_name, o.name)
            }
        };

        match parent_1.cmp(&parent_2) {
            Ordering::Less => Ordering::Less,
            Ordering::Greater => Ordering::Greater,
            Ordering::Equal => sort_field_name(selectable_name_1, selectable_name_2),
        }
    });
    client_types
}

fn sorted_entrypoints<TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
) -> Vec<(
    MemoRef<ClientScalarSelectable<TCompilationProfile>>,
    &EntrypointDeclarationInfo,
)> {
    let mut entrypoints = validated_entrypoints(db)
        .iter()
        .map(
            |(
                (parent_object_entity_name, client_scalar_selectable_name),
                entrypoint_declaration_info,
            )| {
                let entrypoint_declaration_info = entrypoint_declaration_info
                    .as_ref()
                    .expect("Expected entrypoint to be valid.");

                // TODO don't clone, this is only required for lifetime reasons (because
                // we cannot return references with a 'db lifetime)
                let client_scalar_selectable = deprecated_client_scalar_selectable_named(
                    db,
                    *parent_object_entity_name,
                    *client_scalar_selectable_name,
                )
                .to_owned()
                .expect(
                    "Expected parsing to have succeeded by this point. \
                    This is indicative of a bug in Isograph.",
                )
                .expect_selectable_to_exist(
                    *parent_object_entity_name,
                    *client_scalar_selectable_name,
                );
                (client_scalar_selectable, entrypoint_declaration_info)
            },
        )
        .collect::<Vec<_>>();
    entrypoints.sort_by(
        |(client_scalar_selectable_1, _), (client_scalar_selectable_2, _)| {
            let client_scalar_selectable_1 = client_scalar_selectable_1.lookup(db);
            let client_scalar_selectable_2 = client_scalar_selectable_2.lookup(db);

            match client_scalar_selectable_1
                .parent_entity_name
                .cmp(&client_scalar_selectable_2.parent_entity_name)
            {
                Ordering::Less => Ordering::Less,
                Ordering::Greater => Ordering::Greater,
                Ordering::Equal => sort_field_name(
                    client_scalar_selectable_1.name,
                    client_scalar_selectable_2.name,
                ),
            }
        },
    );
    entrypoints
}

fn sort_field_name(field_1: SelectableName, field_2: SelectableName) -> Ordering {
    // We cannot alphabetically sort by field_name. This is because
    // if Query.Foo comes before Query.FooBar in the generated iso.ts,
    // then the iso literal containing field Query.FooBar will be
    // matched with the overload for Query.Foo, which is incorrect.
    //
    // So, instead, we must sort alphabetically, except if a field
    // starts with the other field; then the longer field comes first.
    //
    // TODO confirm that this is a stable sort. It should be, I think!

    let field_1 = field_1.lookup();
    let field_2 = field_2.lookup();

    if field_1.starts_with(field_2) {
        Ordering::Less
    } else if field_2.starts_with(field_1) {
        Ordering::Greater
    } else {
        field_1.cmp(field_2)
    }
}
