import type {IsographEntrypoint, NormalizationAstLoader, RefetchQueryNormalizationArtifactWrapper} from '@isograph/react';
import {Mutation__SetTagline__param} from './param_type';
import {Mutation__SetTagline__output_type} from './output_type';
import readerResolver from './resolver_reader';
import queryText from './query_text';
// import normalizationAst from './normalization_ast';
const nestedRefetchQueries: RefetchQueryNormalizationArtifactWrapper[] = [];

const artifact: IsographEntrypoint<
  Mutation__SetTagline__param,
  Mutation__SetTagline__output_type,
  NormalizationAstLoader
> = {
  kind: "Entrypoint",
  networkRequestInfo: {
    kind: "NetworkRequestInfo",
    queryText,
    normalizationAst: { kind: "NormalizationAstLoader", loader: () => import('./normalization_ast').then(x => x.default) },
  },
  concreteType: "Mutation",
  readerWithRefetchQueries: {
    kind: "ReaderWithRefetchQueries",
    nestedRefetchQueries,
    readerArtifact: readerResolver,
  },
};

export default artifact;
