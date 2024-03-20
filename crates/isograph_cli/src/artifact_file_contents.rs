use std::collections::HashMap;

use common_lang_types::{IsographObjectTypeName, SelectableFieldName};
use isograph_schema::{ClientFieldVariant, ObjectTypeAndFieldNames};

use crate::generate_artifacts::{
    ClientFieldOutputType, EntrypointArtifactInfo, JavaScriptImports, ReaderArtifactInfo,
    RefetchArtifactInfo,
};

impl<'schema> EntrypointArtifactInfo<'schema> {
    pub(crate) fn file_contents(self) -> String {
        let EntrypointArtifactInfo {
            query_text,
            normalization_ast,
            refetch_query_artifact_import,
            query_name,
            parent_type,
        } = self;
        let entrypoint_params_typename = format!("{}__{}__param", parent_type.name, query_name);
        let entrypoint_output_type_name =
            format!("{}__{}__outputType", parent_type.name, query_name);
        format!(
            "import type {{IsographEntrypoint, \
            NormalizationAst, RefetchQueryArtifactWrapper}} from '@isograph/react';\n\
            import type {{{entrypoint_params_typename}, {entrypoint_output_type_name}}} from './reader';\n\
            import readerResolver from './reader';\n\
            {refetch_query_artifact_import}\n\n\
            const queryText = '{query_text}';\n\n\
            const normalizationAst: NormalizationAst = {normalization_ast};\n\
            const artifact: IsographEntrypoint<\n\
            {}{entrypoint_params_typename},\n\
            {}{entrypoint_output_type_name}\n\
            > = {{\n\
            {}kind: \"Entrypoint\",\n\
            {}queryText,\n\
            {}normalizationAst,\n\
            {}nestedRefetchQueries,\n\
            {}readerArtifact: readerResolver,\n\
            }};\n\n\
            export default artifact;\n",
            "  ",
            "  ",
            "  ",
            "  ",
            "  ",
            "  ",
            "  ",
        )
    }
}

impl<'schema> ReaderArtifactInfo<'schema> {
    pub(crate) fn file_contents(self) -> String {
        let ReaderArtifactInfo {
            function_import_statement,
            resolver_parameter_type,
            client_field_output_type,
            reader_ast,
            nested_client_field_artifact_imports,
            parent_type,
            resolver_variant,
            resolver_field_name,
            ..
        } = self;
        let nested_client_field_import_statement = nested_client_field_names_to_import_statement(
            nested_client_field_artifact_imports,
            parent_type.name,
        );
        let output_type_text = get_output_type_text(
            parent_type.name,
            resolver_field_name,
            client_field_output_type,
        );

        // We are not modeling this well, I think.
        let parent_name = parent_type.name;
        let variant = match resolver_variant {
            ClientFieldVariant::Component => {
                format!("{{ kind: \"Component\", componentName: \"{parent_name}.{resolver_field_name}\" }}")
            }
            _ => "{ kind: \"Eager\" }".to_string(),
        };
        let reader_param_type = format!("{parent_name}__{resolver_field_name}__param");
        let reader_output_type = format!("{parent_name}__{resolver_field_name}__outputType");
        format!(
            "import type {{ReaderArtifact, ReaderAst, ExtractSecondParam}} from '@isograph/react';\n\
            {function_import_statement}\n\
            {nested_client_field_import_statement}\n\
            {output_type_text}\n\n\
            const readerAst: ReaderAst<{reader_param_type}> = {reader_ast};\n\n\
            export type {reader_param_type} = {resolver_parameter_type};\n\n\
            const artifact: ReaderArtifact<\n\
            {}{reader_param_type},\n\
            {}{reader_output_type}\n\
            > = {{\n\
            {}kind: \"ReaderArtifact\",\n\
            {}resolver: resolver as any,\n\
            {}readerAst,\n\
            {}variant: {variant},\n\
            }};\n\n\
            export default artifact;\n",
            "  ",
            "  ",
            "  ",
            "  ",
            "  ",
            "  ",
        )
    }
}

impl RefetchArtifactInfo {
    pub(crate) fn file_contents(self) -> String {
        let RefetchArtifactInfo {
            normalization_ast,
            query_text,
            ..
        } = self;

        format!(
            "import type {{IsographEntrypoint, ReaderAst, FragmentReference, NormalizationAst}} from '@isograph/react';\n\
            const queryText = '{query_text}';\n\n\
            const normalizationAst: NormalizationAst = {normalization_ast};\n\
            const artifact: any = {{\n\
            {}kind: \"RefetchQuery\",\n\
            {}queryText,\n\
            {}normalizationAst,\n\
            }};\n\n\
            export default artifact;\n",
            "  ",
            "  ",
            "  ",

        )
    }
}

fn nested_client_field_names_to_import_statement(
    nested_client_field_imports: HashMap<ObjectTypeAndFieldNames, JavaScriptImports>,
    current_file_type_name: IsographObjectTypeName,
) -> String {
    let mut overall = String::new();

    // TODO we should always sort outputs. We should find a nice generic way to ensure that.
    let mut nested_client_field_imports: Vec<_> = nested_client_field_imports.into_iter().collect();
    nested_client_field_imports.sort_by(|(a, _), (b, _)| a.cmp(b));

    for (nested_client_field_name, javascript_import) in nested_client_field_imports {
        write_client_field_import(
            javascript_import,
            nested_client_field_name,
            &mut overall,
            current_file_type_name,
        );
    }
    overall
}

fn write_client_field_import(
    javascript_import: JavaScriptImports,
    nested_resolver_name: ObjectTypeAndFieldNames,
    overall: &mut String,
    current_file_type_name: IsographObjectTypeName,
) {
    if !javascript_import.default_import && javascript_import.types.is_empty() {
        panic!("Resolver imports should not be created in an empty state.");
    }

    let mut s = "import ".to_string();
    if javascript_import.default_import {
        s.push_str(&format!("{}", nested_resolver_name.underscore_separated()));
    }
    let mut types = javascript_import.types.iter();
    if let Some(first) = types.next() {
        if javascript_import.default_import {
            s.push_str(",");
        }
        s.push_str(" { ");
        s.push_str(&format!("{}", first.globally_unique_type_name));
        for value in types {
            s.push_str(&format!(", {}", value.globally_unique_type_name));
        }
        s.push_str("}");
    }
    s.push_str(&format!(
        " from '{}';\n",
        nested_resolver_name.relative_path(current_file_type_name)
    ));
    overall.push_str(&s);
}

fn get_output_type_text(
    parent_type_name: IsographObjectTypeName,
    field_name: SelectableFieldName,
    output_type: ClientFieldOutputType,
) -> String {
    format!(
        "// the type, when read out (either via useLazyReference or via graph)\n\
        export type {}__{}__outputType = {};",
        parent_type_name, field_name, output_type
    )
}
