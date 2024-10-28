use intern::Lookup;
use std::{cmp::Ordering, path::PathBuf};

use common_lang_types::{ArtifactPathAndContent, SelectableFieldName};
use isograph_schema::{
    ClientFieldVariant, UserWrittenComponentVariant, ValidatedClientField, ValidatedSchema,
};

use crate::generate_artifacts::ISO_TS;

fn build_iso_overload_for_entrypoint(
    validated_client_field: &ValidatedClientField,
) -> (String, String) {
    let mut s: String = "".to_string();
    let import = format!(
        "import entrypoint_{} from '../__isograph/{}/{}/entrypoint';\n",
        validated_client_field.type_and_field.underscore_separated(),
        validated_client_field.type_and_field.type_name,
        validated_client_field.type_and_field.field_name,
    );
    let formatted_field = format!(
        "entrypoint {}.{}",
        validated_client_field.type_and_field.type_name,
        validated_client_field.type_and_field.field_name
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

fn build_iso_overload_for_client_defined_field(
    client_field_and_variant: (&ValidatedClientField, UserWrittenComponentVariant),
) -> (String, String) {
    let (client_field, variant) = client_field_and_variant;
    let mut s: String = "".to_string();
    let import = format!(
        "import {{ type {}__param }} from './{}/{}/param_type';\n",
        client_field.type_and_field.underscore_separated(),
        client_field.type_and_field.type_name,
        client_field.type_and_field.field_name,
    );
    let formatted_field = format!(
        "field {}.{}",
        client_field.type_and_field.type_name, client_field.type_and_field.field_name
    );
    if matches!(variant, UserWrittenComponentVariant::Component) {
        s.push_str(&format!(
            "
export function iso<T>(
  param: T & MatchesWhitespaceAndString<'{}', T>
): IdentityWithParamComponent<{}__param>;\n",
            formatted_field,
            client_field.type_and_field.underscore_separated(),
        ));
    } else {
        s.push_str(&format!(
            "
export function iso<T>(
  param: T & MatchesWhitespaceAndString<'{}', T>
): IdentityWithParam<{}__param>;\n",
            formatted_field,
            client_field.type_and_field.underscore_separated(),
        ));
    }
    (import, s)
}

pub(crate) fn build_iso_overload_artifact(schema: &ValidatedSchema) -> ArtifactPathAndContent {
    let mut imports = "import type { IsographEntrypoint } from '@isograph/react';\n".to_string();
    let mut content = String::from(
        "
// This is the type given to regular client fields.
// This means that the type of the exported iso literal is exactly
// the type of the passed-in function, which takes one parameter
// of type TParam.
type IdentityWithParam<TParam extends object> = <TClientFieldReturn>(
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

    let client_defined_field_overloads = sorted_user_written_fields(schema)
        .into_iter()
        .map(build_iso_overload_for_client_defined_field);
    for (import, field_overload) in client_defined_field_overloads {
        imports.push_str(&import);
        content.push_str(&field_overload);
    }

    let entrypoint_overloads = sorted_entrypoints(schema)
        .into_iter()
        .map(build_iso_overload_for_entrypoint);
    for (import, entrypoint_overload) in entrypoint_overloads {
        imports.push_str(&import);
        content.push_str(&entrypoint_overload);
    }

    content.push_str(
        "
export function iso(_isographLiteralText: string):
  | IdentityWithParam<any>
  | IdentityWithParamComponent<any>
  | IsographEntrypoint<any, any>
{
  throw new Error('iso: Unexpected invocation at runtime. Either the Babel transform ' +
      'was not set up, or it failed to identify this call site. Make sure it ' +
      'is being used verbatim as `iso`.');
}",
    );
    imports.push_str(&content);
    ArtifactPathAndContent {
        file_content: imports,
        relative_directory: PathBuf::new(),
        file_name_prefix: *ISO_TS,
    }
}

fn sorted_user_written_fields(
    schema: &ValidatedSchema,
) -> Vec<(&ValidatedClientField, UserWrittenComponentVariant)> {
    let mut fields = user_written_fields(schema).collect::<Vec<_>>();
    fields.sort_by(|client_field_1, client_field_2| {
        match client_field_1
            .0
            .type_and_field
            .type_name
            .cmp(&client_field_2.0.type_and_field.type_name)
        {
            Ordering::Less => Ordering::Less,
            Ordering::Greater => Ordering::Greater,
            Ordering::Equal => sort_field_name(
                client_field_1.0.type_and_field.field_name,
                client_field_2.0.type_and_field.field_name,
            ),
        }
    });
    fields
}

fn sorted_entrypoints(schema: &ValidatedSchema) -> Vec<&ValidatedClientField> {
    let mut entrypoints = schema
        .entrypoints
        .iter()
        .map(|client_field_id| schema.client_field(*client_field_id))
        .collect::<Vec<_>>();
    entrypoints.sort_by(|client_field_1, client_field_2| {
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

fn sort_field_name(field_1: SelectableFieldName, field_2: SelectableFieldName) -> Ordering {
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

fn user_written_fields(
    schema: &ValidatedSchema,
) -> impl Iterator<Item = (&ValidatedClientField, UserWrittenComponentVariant)> + '_ {
    schema
        .client_fields
        .iter()
        .filter_map(|client_field| match client_field.variant {
            ClientFieldVariant::ClientPointer(_) => None,
            ClientFieldVariant::UserWritten(info) => {
                Some((client_field, info.user_written_component_variant))
            }
            ClientFieldVariant::ImperativelyLoadedField(_) => None,
        })
}
