use intern::Lookup;
use isograph_config::GenerateFileExtensionsOption;
use isograph_lang_types::{ClientFieldDirectiveSet, SelectionType};
use std::{cmp::Ordering, collections::BTreeSet};

use common_lang_types::{ArtifactPathAndContent, SelectableName, ServerObjectEntityName};
use isograph_schema::{
    ClientScalarOrObjectSelectable, ClientScalarSelectable, ClientSelectable,
    EntrypointDeclarationInfo, NetworkProtocol, Schema,
};

use crate::generate_artifacts::{ISO_TS_FILE_NAME, print_javascript_type_declaration};

fn build_iso_overload_for_entrypoint<TNetworkProtocol: NetworkProtocol>(
    validated_client_field: &ClientScalarSelectable<TNetworkProtocol>,
    file_extensions: GenerateFileExtensionsOption,
) -> (String, String) {
    let formatted_field = format!(
        "entrypoint {}.{}",
        validated_client_field.type_and_field.type_name,
        validated_client_field.type_and_field.field_name
    );
    let mut s: String = "".to_string();
    let import = format!(
        "import entrypoint_{} from '../__isograph/{}/{}/entrypoint{}';\n",
        validated_client_field.type_and_field.underscore_separated(),
        validated_client_field.type_and_field.type_name,
        validated_client_field.type_and_field.field_name,
        file_extensions.ts()
    );

    s.push_str(&format!(
        "
export function iso<T>(
  param: T & MatchesWhitespaceAndString<'{}', T>
): typeof entrypoint_{};\n",
        formatted_field,
        validated_client_field.type_and_field.underscore_separated(),
    ));
    (import, s)
}

fn build_iso_overload_for_client_defined_type<TNetworkProtocol: NetworkProtocol>(
    client_type_and_variant: (ClientSelectable<TNetworkProtocol>, ClientFieldDirectiveSet),
    file_extensions: GenerateFileExtensionsOption,
    link_types: &mut BTreeSet<ServerObjectEntityName>,
) -> (String, String) {
    let (client_type, variant) = client_type_and_variant;
    let mut s: String = "".to_string();
    let import = format!(
        "import {{ type {}__param }} from './{}/{}/param_type{}';\n",
        client_type.type_and_field().underscore_separated(),
        client_type.type_and_field().type_name,
        client_type.type_and_field().field_name,
        file_extensions.ts()
    );

    let formatted_field = format!(
        "{} {}.{}",
        match client_type {
            SelectionType::Scalar(_) => "field",
            SelectionType::Object(_) => "pointer",
        },
        client_type.type_and_field().type_name,
        client_type.type_and_field().field_name
    );
    if matches!(variant, ClientFieldDirectiveSet::Component(_)) {
        s.push_str(&format!(
            "
export function iso<T>(
  param: T & MatchesWhitespaceAndString<'{}', T>
): IdentityWithParamComponent<{}__param>;\n",
            formatted_field,
            client_type.type_and_field().underscore_separated(),
        ));
    } else if let SelectionType::Object(client_pointer) = client_type {
        link_types.insert(*client_pointer.target_object_entity_name.inner());
        s.push_str(&format!(
            "
export function iso<T>(
  param: T & MatchesWhitespaceAndString<'{}', T>
): IdentityWithParam<{}__param, {}>;\n",
            formatted_field,
            client_type.type_and_field().underscore_separated(),
            print_javascript_type_declaration(
                &client_pointer.target_object_entity_name.clone().map(
                    &mut |target_object_entity_name| {
                        format!("{}__link__output_type", &target_object_entity_name)
                    }
                )
            )
        ));
    } else {
        s.push_str(&format!(
            "
export function iso<T>(
  param: T & MatchesWhitespaceAndString<'{}', T>
): IdentityWithParam<{}__param>;\n",
            formatted_field,
            client_type.type_and_field().underscore_separated(),
        ));
    }
    (import, s)
}

pub(crate) fn build_iso_overload_artifact<TNetworkProtocol: NetworkProtocol>(
    schema: &Schema<TNetworkProtocol>,
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
        sorted_user_written_types(schema)
            .into_iter()
            .map(|client_type| {
                build_iso_overload_for_client_defined_type(
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
        imports.push_str(&format!(
            "import {{ type {}__link__output_type }} from './{}/link/output_type{}';\n",
            target_object_entity_name,
            target_object_entity_name,
            file_extensions.ts()
        ));
    }

    let entrypoint_overloads = sorted_entrypoints(schema)
        .into_iter()
        .map(|(field, _)| build_iso_overload_for_entrypoint(field, file_extensions));
    for (import, entrypoint_overload) in entrypoint_overloads {
        imports.push_str(&import);
        content.push_str(&entrypoint_overload);
    }

    (match no_babel_transform {
        false => {
            content.push_str(
                "
export function iso(_isographLiteralText: string):
  | IdentityWithParam<any>
  | IdentityWithParamComponent<any>
  | IsographEntrypoint<any, any, any>
{\n",
            );
            content.push_str("  throw new Error('iso: Unexpected invocation at runtime. Either the Babel transform ' +
      'was not set up, or it failed to identify this call site. Make sure it ' +
      'is being used verbatim as `iso`. If you cannot use the babel transform, ' + 
      'set options.no_babel_transform to true in your Isograph config. ');\n}")
        }
        true => {
            let switch_cases = sorted_entrypoints(schema).into_iter().map(
                |(field, entrypoint_declaration_info)| {
                    format!(
                        "    case '{}':
      return entrypoint_{};\n",
                        entrypoint_declaration_info.iso_literal_text,
                        field.type_and_field.underscore_separated()
                    )
                },
            );

            content.push_str(
                "
export function iso(isographLiteralText: string):
  | IdentityWithParam<any>
  | IdentityWithParamComponent<any>
  | IsographEntrypoint<any, any, any>
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
    });

    imports.push_str(&content);
    ArtifactPathAndContent {
        file_content: imports,
        file_name: *ISO_TS_FILE_NAME,
        type_and_field: None,
    }
}

fn sorted_user_written_types<TNetworkProtocol: NetworkProtocol>(
    schema: &Schema<TNetworkProtocol>,
) -> Vec<(
    ClientSelectable<'_, TNetworkProtocol>,
    ClientFieldDirectiveSet,
)> {
    let mut client_types = schema
        .user_written_client_types()
        .map(|x| (x.1, x.2))
        .collect::<Vec<_>>();
    client_types.sort_by(|client_type_1, client_type_2| {
        match client_type_1
            .0
            .type_and_field()
            .type_name
            .cmp(&client_type_2.0.type_and_field().type_name)
        {
            Ordering::Less => Ordering::Less,
            Ordering::Greater => Ordering::Greater,
            Ordering::Equal => sort_field_name(
                client_type_1.0.type_and_field().field_name,
                client_type_2.0.type_and_field().field_name,
            ),
        }
    });
    client_types
}

fn sorted_entrypoints<TNetworkProtocol: NetworkProtocol>(
    schema: &Schema<TNetworkProtocol>,
) -> Vec<(
    &ClientScalarSelectable<TNetworkProtocol>,
    &EntrypointDeclarationInfo,
)> {
    let mut entrypoints = schema
        .entrypoints
        .iter()
        .map(
            |((parent_object_entity_name, client_field_name), entrypoint_declaration_info)| {
                (
                    schema
                        .client_field(*parent_object_entity_name, *client_field_name)
                        .expect(
                            "Expected selectable to exist. \
                            This is indicative of a bug in Isograph.",
                        ),
                    entrypoint_declaration_info,
                )
            },
        )
        .collect::<Vec<_>>();
    entrypoints.sort_by(|(client_field_1, _), (client_field_2, _)| {
        match client_field_1
            .type_and_field
            .type_name
            .cmp(&client_field_2.type_and_field.type_name)
        {
            Ordering::Less => Ordering::Less,
            Ordering::Greater => Ordering::Greater,
            Ordering::Equal => sort_field_name(
                client_field_1.type_and_field.field_name,
                client_field_2.type_and_field.field_name,
            ),
        }
    });
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
