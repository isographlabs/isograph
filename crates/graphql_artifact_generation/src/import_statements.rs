use std::collections::{BTreeMap, BTreeSet};

use isograph_schema::ObjectTypeAndFieldName;

#[derive(Clone, Copy, Debug)]
pub(crate) enum ResolverReaderOrRefetchResolver {
    ResolverReader,
    RefetchReader,
}

impl ResolverReaderOrRefetchResolver {
    pub fn filename(&self) -> &'static str {
        match self {
            ResolverReaderOrRefetchResolver::ResolverReader => "resolver_reader",
            ResolverReaderOrRefetchResolver::RefetchReader => "refetch_reader",
        }
    }
}

pub(crate) type ReaderImports = BTreeMap<ObjectTypeAndFieldName, ResolverReaderOrRefetchResolver>;
pub(crate) type ParamTypeImports = BTreeSet<ObjectTypeAndFieldName>;

pub(crate) fn reader_imports_to_import_statement(reader_imports: &ReaderImports) -> String {
    let mut output = String::new();
    for (type_and_field, artifact_type) in reader_imports.iter() {
        output.push_str(&format!(
            "import {}__{} from '../../{}/{}/{}';\n",
            type_and_field.underscore_separated(),
            artifact_type.filename(),
            type_and_field.type_name,
            type_and_field.field_name,
            artifact_type.filename()
        ));
    }
    output
}

pub(crate) fn param_type_imports_to_import_statement(
    param_type_imports: &ParamTypeImports,
) -> String {
    let mut output = String::new();
    for type_and_field in param_type_imports.iter() {
        output.push_str(&format!(
            "import {{ type {}__output_type }} from '../../{}/{}/output_type';\n",
            type_and_field.underscore_separated(),
            type_and_field.type_name,
            type_and_field.field_name,
        ));
    }
    output
}
