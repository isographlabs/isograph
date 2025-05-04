import type {IsographEntrypoint, NormalizationAstLoader, RefetchQueryNormalizationArtifactWrapper} from '@isograph/react';
import {Image__ImageDisplay__param} from './param_type';
import {Image__ImageDisplay__output_type} from './output_type';
import readerResolver from './resolver_reader';
import queryText from './query_text';
// import normalizationAst from './normalization_ast';
const nestedRefetchQueries: RefetchQueryNormalizationArtifactWrapper[] = [];

const artifact: IsographEntrypoint<
  Image__ImageDisplay__param,
  Image__ImageDisplay__output_type,
  NormalizationAstLoader
> = {
  kind: "Entrypoint",
  networkRequestInfo: {
    kind: "NetworkRequestInfo",
    queryText,
    normalizationAst: { kind: "NormalizationAstLoader", loader: () => import('./normalization_ast').then(x => x.default) },
  },
  concreteType: "Query",
  readerWithRefetchQueries: {
    kind: "ReaderWithRefetchQueries",
    nestedRefetchQueries,
    readerArtifact: readerResolver,
  },
};

export default artifact;
