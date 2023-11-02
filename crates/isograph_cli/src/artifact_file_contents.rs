use std::collections::HashMap;

use common_lang_types::IsographObjectTypeName;
use isograph_schema::ResolverTypeAndField;

use crate::generate_artifacts::{
    EntrypointArtifact, ReaderArtifact, RefetchArtifact, ResolverImport, ResolverReadOutType,
};

impl<'schema> EntrypointArtifact<'schema> {
    pub(crate) fn file_contents(self) -> String {
        let EntrypointArtifact {
            query_text,
            normalization_ast,
            refetch_query_artifact_import,
            ..
        } = self;

        format!(
            "import type {{IsographFetchableResolver, FragmentReference, \
            NormalizationAst, RefetchQueryArtifactWrapper}} from '@isograph/react';\n\
            import type {{ReadFromStoreType, ResolverParameterType, ReadOutType}} from './reader.isograph';\n\
            import readerResolver from './reader.isograph';\n\
            {refetch_query_artifact_import}\n\n\
            const queryText = '{query_text}';\n\n\
            const normalizationAst: NormalizationAst = {normalization_ast};\n\
            const artifact: IsographFetchableResolver<ReadFromStoreType, ResolverParameterType, ReadOutType> = {{\n\
            {}kind: 'FetchableResolver',\n\
            {}queryText,\n\
            {}normalizationAst,\n\
            {}nestedRefetchQueries,\n\
            {}readerAst: readerResolver.readerAst,\n\
            {}resolver: readerResolver.resolver,\n\
            }};\n\n\
            export default artifact;\n",
            "  ", "  ", "  ", "  ", "  ", "  ",
        )
    }
}

impl<'schema> ReaderArtifact<'schema> {
    pub(crate) fn file_contents(self) -> String {
        let ReaderArtifact {
            resolver_import_statement,
            resolver_parameter_type,
            resolver_return_type,
            resolver_read_out_type,
            reader_ast,
            nested_resolver_artifact_imports,
            parent_type,
            ..
        } = self;
        let nested_resolver_import_statement = nested_resolver_names_to_import_statement(
            nested_resolver_artifact_imports,
            parent_type.name,
        );
        let read_out_type_text = get_read_out_type_text(resolver_read_out_type);

        format!(
            "import type {{IsographNonFetchableResolver, ReaderAst}} from '@isograph/react';\n\
            {resolver_import_statement}\n\
            {nested_resolver_import_statement}\n\
            {read_out_type_text}\n\n\
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

impl RefetchArtifact {
    pub(crate) fn file_contents(self) -> String {
        let RefetchArtifact {
            normalization_ast,
            query_text,
            ..
        } = self;

        format!(
            "import type {{IsographFetchableResolver, ReaderAst, FragmentReference, NormalizationAst}} from '@isograph/react';\n\
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

fn nested_resolver_names_to_import_statement(
    nested_resolver_imports: HashMap<ResolverTypeAndField, ResolverImport>,
    current_file_type_name: IsographObjectTypeName,
) -> String {
    let mut overall = String::new();

    // TODO we should always sort outputs. We should find a nice generic way to ensure that.
    let mut nested_resolver_imports: Vec<_> = nested_resolver_imports.into_iter().collect();
    nested_resolver_imports.sort_by(|(a, _), (b, _)| a.cmp(b));

    for (nested_resolver_name, resolver_import) in nested_resolver_imports {
        write_resolver_import(
            resolver_import,
            nested_resolver_name,
            &mut overall,
            current_file_type_name,
        );
    }
    overall
}

fn write_resolver_import(
    resolver_import: ResolverImport,
    nested_resolver_name: ResolverTypeAndField,
    overall: &mut String,
    current_file_type_name: IsographObjectTypeName,
) {
    if !resolver_import.default_import && resolver_import.types.is_empty() {
        panic!("Resolver imports should not be created in an empty state.");
    }

    let mut s = "import ".to_string();
    if resolver_import.default_import {
        s.push_str(&format!("{}", nested_resolver_name.underscore_separated()));
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
    s.push_str(&format!(
        " from '{}';\n",
        nested_resolver_name.relative_path(current_file_type_name)
    ));
    overall.push_str(&s);
}

fn get_read_out_type_text(read_out_type: ResolverReadOutType) -> String {
    format!("// the type, when read out (either via useLazyReference or via graph)\nexport type ReadOutType = {};", read_out_type)
}
