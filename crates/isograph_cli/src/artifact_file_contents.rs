use std::collections::HashMap;

use common_lang_types::TypeAndField;

use crate::generate_artifacts::{
    FetchableResolver, NonFetchableResolver, ResolverImport, ResolverReadOutType,
};

impl<'schema> FetchableResolver<'schema> {
    pub(crate) fn file_contents(self) -> String {
        let FetchableResolver {
            query_text,
            resolver_import_statement,
            resolver_parameter_type,
            resolver_return_type,
            resolver_read_out_type,
            reader_ast,
            nested_resolver_artifact_imports,
            convert_function,
            ..
        } = self;
        let read_out_type_text = get_read_out_type_text(resolver_read_out_type);
        let nested_resolver_artifact_imports =
            nested_resolver_names_to_import_statement(nested_resolver_artifact_imports);

        format!(
            "import type {{IsographFetchableResolver, ReaderAst, FragmentReference}} from '@isograph/react';\n\
            import {{ getRefRendererForName }} from '@isograph/react';\n\
            {resolver_import_statement}\n\
            {nested_resolver_artifact_imports}\n\
            const queryText = '{query_text}';\n\n\
            // TODO support changing this,\n\
            export type ReadFromStoreType = ResolverParameterType;\n\n\
            const normalizationAst = {{notNeededForDemo: true}};\n\
            const readerAst: ReaderAst<ReadFromStoreType> = {reader_ast};\n\n\
            export type ResolverParameterType = {resolver_parameter_type};\n\n\
            // The type, when returned from the resolver\n\
            export type ResolverReturnType = {resolver_return_type};\n\n\
            {read_out_type_text}\n\n\
            const artifact: IsographFetchableResolver<ReadFromStoreType, ResolverParameterType, ReadOutType> = {{\n\
            {}kind: 'FetchableResolver',\n\
            {}queryText,\n\
            {}normalizationAst,\n\
            {}readerAst,\n\
            {}resolver: resolver as any,\n\
            {}convert: {convert_function},\n\
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

impl<'schema> NonFetchableResolver<'schema> {
    pub(crate) fn file_contents(self) -> String {
        let NonFetchableResolver {
            resolver_import_statement,
            resolver_parameter_type,
            resolver_return_type,
            resolver_read_out_type,
            reader_ast,
            nested_resolver_artifact_imports,
            ..
        } = self;
        let nested_resolver_import_statement =
            nested_resolver_names_to_import_statement(nested_resolver_artifact_imports);
        let read_out_type_text = get_read_out_type_text(resolver_read_out_type);

        format!(
            "import type {{IsographNonFetchableResolver, ReaderAst}} from '@isograph/react';\n\
            {resolver_import_statement}\n\
            {nested_resolver_import_statement}\n\
            {read_out_type_text}\n\n\
            // TODO support changing this\n\
            export type ReadFromStoreType = ResolverParameterType;\n\n\
            const readerAst: ReaderAst<ReadFromStoreType> = {reader_ast};\n\n\
            export type ResolverParameterType = {resolver_parameter_type};\n\n\
            // The type, when returned from the resolver\n\
            export type ResolverReturnType = {resolver_return_type};\n\n\
            const artifact: IsographNonFetchableResolver<ReadFromStoreType, ResolverParameterType, ReadOutType> = {{\n\
            {}kind: 'NonFetchableResolver',\n\
            {}resolver: resolver as any,\n\
            {}readerAst,\n\
            }};\n\n\
            export default artifact;\n",
            "  ",
            "  ",
            "  ",
        )
    }
}

fn nested_resolver_names_to_import_statement(
    nested_resolver_imports: HashMap<TypeAndField, ResolverImport>,
) -> String {
    let mut overall = String::new();

    // TODO we should always sort outputs. We should find a nice generic way to ensure that.
    let mut nested_resolver_imports: Vec<_> = nested_resolver_imports.into_iter().collect();
    nested_resolver_imports.sort_by(|(a, _), (b, _)| a.cmp(b));

    for (nested_resolver_name, resolver_import) in nested_resolver_imports {
        if !resolver_import.default_import && resolver_import.types.is_empty() {
            continue;
        }

        let mut s = "import ".to_string();
        if resolver_import.default_import {
            s.push_str(&format!("{}", nested_resolver_name));
        }
        let mut types = resolver_import.types.iter();
        if let Some(first) = types.next() {
            if resolver_import.default_import {
                s.push_str(",");
            }
            s.push_str(" { ");
            s.push_str(&format!("{} as {} ", first.original, first.alias));
            for value in types {
                s.push_str(&format!(", {} as {} ", value.original, value.alias));
            }
            s.push_str("}");
        }
        s.push_str(&format!(" from './{}.isograph';\n", nested_resolver_name));
        overall.push_str(&s);
    }
    overall
}

fn get_read_out_type_text(read_out_type: ResolverReadOutType) -> String {
    format!("// the type, when read out (either via useLazyReference or via graph)\nexport type ReadOutType = {};", read_out_type)
}
