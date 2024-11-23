use std::collections::BTreeSet;

use isograph_config::GenerateFileExtensionsOption;
use isograph_schema::ObjectTypeAndFieldName;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum ImportedFileCategory {
    ResolverReader,
    RefetchReader,
    Entrypoint,
}

impl ImportedFileCategory {
    pub fn filename(&self) -> &'static str {
        match self {
            ImportedFileCategory::ResolverReader => "resolver_reader",
            ImportedFileCategory::RefetchReader => "refetch_reader",
            ImportedFileCategory::Entrypoint => "entrypoint",
        }
    }
}

pub(crate) type ReaderImports = BTreeSet<(ObjectTypeAndFieldName, ImportedFileCategory)>;
pub(crate) type ParamTypeImports = BTreeSet<ObjectTypeAndFieldName>;
pub(crate) type LinkImports = bool;

pub(crate) fn reader_imports_to_import_statement(
    reader_imports: &ReaderImports,
    file_extensions: GenerateFileExtensionsOption,
) -> String {
    let mut output = String::new();
    for (type_and_field, artifact_type) in reader_imports.iter() {
        output.push_str(&format!(
            "import {}__{} from '../../{}/{}/{}{}';\n",
            type_and_field.underscore_separated(),
            artifact_type.filename(),
            type_and_field.type_name,
            type_and_field.field_name,
            artifact_type.filename(),
            file_extensions.ts()
        ));
    }
    output
}

pub(crate) fn param_type_imports_to_import_statement(
    param_type_imports: &ParamTypeImports,
    file_extensions: GenerateFileExtensionsOption,
) -> String {
    let mut output = String::new();
    for type_and_field in param_type_imports.iter() {
        output.push_str(&format!(
            "import {{ type {}__output_type }} from '../../{}/{}/output_type{}';\n",
            type_and_field.underscore_separated(),
            type_and_field.type_name,
            type_and_field.field_name,
            file_extensions.ts(),
        ));
    }
    output
}

pub(crate) fn param_type_imports_to_import_param_statement(
    param_type_imports: &ParamTypeImports,
    file_extensions: GenerateFileExtensionsOption,
) -> String {
    let mut output = String::new();
    for type_and_field in param_type_imports.iter() {
        output.push_str(&format!(
            "import {{ type {}__param }} from '../../{}/{}/param_type{}';\n",
            type_and_field.underscore_separated(),
            type_and_field.type_name,
            type_and_field.field_name,
            file_extensions.ts()
        ));
    }
    output
}
