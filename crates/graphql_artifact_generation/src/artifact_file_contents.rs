use std::{collections::HashMap, path::PathBuf};

use common_lang_types::{
    ArtifactFileType, IsographObjectTypeName, PathAndContent, SelectableFieldName,
};
use intern::string_key::Intern;
use isograph_schema::ObjectTypeAndFieldNames;
use lazy_static::lazy_static;

use crate::generate_artifacts::{
    ClientFieldFunctionImportStatement, ClientFieldOutputType, JavaScriptImports,
    RefetchReaderArtifactInfo,
};

lazy_static! {
    pub static ref READER: ArtifactFileType = "reader".intern().into();
    pub static ref READER_PARAM_TYPE: ArtifactFileType = "param_type".intern().into();
    pub static ref READER_OUTPUT_TYPE: ArtifactFileType = "output_type".intern().into();
    pub static ref ENTRYPOINT: ArtifactFileType = "entrypoint".intern().into();
    pub static ref ISO_TS: ArtifactFileType = "iso".intern().into();
}

impl<'schema> RefetchReaderArtifactInfo<'schema> {
    pub(crate) fn file_contents(self, relative_directory: &PathBuf) -> Vec<PathAndContent> {
        let RefetchReaderArtifactInfo {
            function_import_statement,
            client_field_parameter_type,
            client_field_output_type,
            reader_ast,
            nested_client_field_artifact_imports,
            parent_type,
            client_field_name,
            ..
        } = self;

        let (client_field_import_statement, client_field_type_import_statement) =
            nested_client_field_names_to_import_statement(
                nested_client_field_artifact_imports,
                parent_type.name,
            );

        let output_type_text = get_output_type_text(
            &function_import_statement,
            parent_type.name,
            client_field_name,
            client_field_output_type,
        );
        let output_type_text = format!(
            "import {{ RefetchQueryNormalizationArtifact }} from '@isograph/react';\n\
            {output_type_text}"
        );

        let parent_name = parent_type.name;
        let reader_param_type = format!("{parent_name}__{client_field_name}__param");

        let reader_content = format!(
            "import type {{RefetchReaderArtifact, ReaderAst, RefetchQueryNormalizationArtifact}} from '@isograph/react';\n\
            import {{ {reader_param_type} }} from './param_type';\n\
            {function_import_statement}\n\
            {client_field_import_statement}\n\
            const readerAst: ReaderAst<{reader_param_type}> = {reader_ast};\n\n\
            const artifact: RefetchReaderArtifact = {{\n\
            {}kind: \"RefetchReaderArtifact\",\n\
            {}// @ts-ignore\n\
            {}resolver,\n\
            {}readerAst,\n\
            }};\n\n\
            export default artifact;\n",
            "  ", "  ", "  ", "  "
        );

        let param_type_content = format!(
            "{client_field_type_import_statement}\n\
            export type {reader_param_type} = {client_field_parameter_type};\n",
        );

        vec![
            PathAndContent {
                relative_directory: relative_directory.clone(),
                file_name_prefix: *READER,
                file_content: reader_content,
            },
            PathAndContent {
                relative_directory: relative_directory.clone(),
                file_name_prefix: *READER_PARAM_TYPE,
                file_content: param_type_content,
            },
            PathAndContent {
                relative_directory: relative_directory.clone(),
                file_name_prefix: *READER_OUTPUT_TYPE,
                file_content: output_type_text,
            },
        ]
    }
}

pub(crate) fn nested_client_field_names_to_import_statement(
    nested_client_field_imports: HashMap<ObjectTypeAndFieldNames, JavaScriptImports>,
    current_file_type_name: IsographObjectTypeName,
) -> (String, String) {
    let mut client_field_import_statement = String::new();
    let mut client_field_type_import_statement = String::new();

    // TODO we should always sort outputs. We should find a nice generic way to ensure that.
    let mut nested_client_field_imports: Vec<_> = nested_client_field_imports.into_iter().collect();
    nested_client_field_imports.sort_by(|(a, _), (b, _)| a.cmp(b));

    for (nested_client_field_name, javascript_import) in nested_client_field_imports {
        write_client_field_import(
            javascript_import,
            nested_client_field_name,
            &mut client_field_import_statement,
            &mut client_field_type_import_statement,
            current_file_type_name,
        );
    }
    (
        client_field_import_statement,
        client_field_type_import_statement,
    )
}

fn write_client_field_import(
    javascript_import: JavaScriptImports,
    nested_client_field_name: ObjectTypeAndFieldNames,
    client_field_import_statement: &mut String,
    client_field_type_import_statement: &mut String,
    current_file_type_name: IsographObjectTypeName,
) {
    if !javascript_import.default_import && javascript_import.types.is_empty() {
        panic!(
            "Client field imports should not be created in an empty state. \
            This is indicative of a bug in Isograph."
        );
    }

    let mut s_client_field_import = "".to_string();
    let mut s_client_field_type_import = "".to_string();

    if javascript_import.default_import {
        s_client_field_import.push_str(&format!(
            "import {} from '{}';\n",
            nested_client_field_name.underscore_separated(),
            nested_client_field_name.relative_path(current_file_type_name, *READER)
        ));
    }

    let mut types = javascript_import.types.iter();
    if let Some(first) = types.next() {
        s_client_field_type_import.push_str(&format!("import {{{}", first));
        for value in types {
            s_client_field_type_import.push_str(&format!(", {}", value));
        }
        s_client_field_type_import.push_str(&format!(
            "}} from '{}';\n",
            nested_client_field_name.relative_path(current_file_type_name, *READER_OUTPUT_TYPE)
        ));
    }

    client_field_import_statement.push_str(&s_client_field_import);
    client_field_type_import_statement.push_str(&s_client_field_type_import);
}

pub(crate) fn get_output_type_text(
    function_import_statement: &ClientFieldFunctionImportStatement,
    parent_type_name: IsographObjectTypeName,
    field_name: SelectableFieldName,
    output_type: ClientFieldOutputType,
) -> String {
    let function_import_statement = &function_import_statement.0;
    format!(
        "{function_import_statement}\n\
        export type {}__{}__outputType = {};",
        parent_type_name, field_name, output_type
    )
}
