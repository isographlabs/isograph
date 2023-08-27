use std::collections::HashMap;

use common_lang_types::TypeAndField;

use crate::generate_artifacts::{
    FetchableResolver, NonFetchableResolver, ResolverImport, ResolverReadOutType,
};

impl<'schema> FetchableResolver<'schema> {
    pub(crate) fn file_contents(self) -> String {
        // TODO don't use merged, use regular selection set when generating fragment type
        // (i.e. we are not data masking)
        format!(
            "import type {{IsographFetchableResolver, ReaderAst, FragmentReference}} from '@isograph/react';\n\
            import {{ getRefRendererForName }} from '@isograph/react';\n\
            {}\n\
            {}\n\
            const queryText = '{}';\n\n\
            // TODO support changing this,\n\
            export type ReadFromStoreType = ResolverParameterType;\n\n\
            const normalizationAst = {{notNeededForDemo: true}};\n\
            const readerAst: ReaderAst<ReadFromStoreType> = {};\n\n\
            export type ResolverParameterType = {};\n\n\
            // The type, when returned from the resolver\n\
            export type ResolverReturnType = {};\n\n\
            {}\n\n\
            const artifact: IsographFetchableResolver<ReadFromStoreType, ResolverParameterType, ReadOutType> = {{\n\
            {}kind: 'FetchableResolver',\n\
            {}queryText,\n\
            {}normalizationAst,\n\
            {}readerAst,\n\
            {}resolver: resolver as any,\n\
            {}convert: {},\n\
            }};\n\n\
            export default artifact;\n",
            self.resolver_import_statement.0,
            nested_resolver_names_to_import_statement(self.nested_resolver_artifact_imports),
            self.query_text.0,
            self.reader_ast.0,
            self.resolver_parameter_type.0,
            self.resolver_return_type.0,
            get_read_out_type_text(self.resolver_read_out_type),
            "  ",
            "  ",
            "  ",
            "  ",
            "  ",
            "  ",
            self.convert_function.0,
        )
    }
}

impl<'schema> NonFetchableResolver<'schema> {
    pub(crate) fn file_contents(self) -> String {
        format!(
            "import type {{IsographNonFetchableResolver, ReaderAst}} from '@isograph/react';\n\
            {}\n\
            {}\n\
            {}\n\n\
            // TODO support changing this\n\
            export type ReadFromStoreType = ResolverParameterType;\n\n\
            const readerAst: ReaderAst<ReadFromStoreType> = {};\n\n\
            export type ResolverParameterType = {};\n\n\
            // The type, when returned from the resolver\n\
            export type ResolverReturnType = {};\n\n\
            const artifact: IsographNonFetchableResolver<ReadFromStoreType, ResolverParameterType, ReadOutType> = {{\n\
            {}kind: 'NonFetchableResolver',\n\
            {}resolver: resolver as any,\n\
            {}readerAst,\n\
            }};\n\n\
            export default artifact;\n",
            self.resolver_import_statement.0,
            nested_resolver_names_to_import_statement(self.nested_resolver_artifact_imports),
            get_read_out_type_text(self.resolver_read_out_type),
            self.reader_ast.0,
            self.resolver_parameter_type.0,
            self.resolver_return_type.0,
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
            s.push_str(&format!("{} as {} ", first.original.0, first.alias.0));
            for value in types {
                s.push_str(&format!(", {} as {} ", value.original.0, value.alias.0));
            }
            s.push_str("}");
        }
        s.push_str(&format!(" from './{}.isograph';\n", nested_resolver_name));
        overall.push_str(&s);
    }
    overall
}

fn get_read_out_type_text(read_out_type: ResolverReadOutType) -> String {
    format!("// the type, when read out (either via useLazyReference or via graph)\nexport type ReadOutType = {};", read_out_type.0)
}
