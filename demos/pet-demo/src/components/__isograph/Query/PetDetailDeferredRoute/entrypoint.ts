import type {IsographEntrypoint, NormalizationAstLoader, RefetchQueryNormalizationArtifactWrapper} from '@isograph/react';
import {Query__PetDetailDeferredRoute__param} from './param_type';
import {Query__PetDetailDeferredRoute__output_type} from './output_type';
import readerResolver from './resolver_reader';
import queryText from './query_text';
// import normalizationAst from './normalization_ast';
const nestedRefetchQueries: RefetchQueryNormalizationArtifactWrapper[] = [];

const artifact: IsographEntrypoint<
  Query__PetDetailDeferredRoute__param,
  Query__PetDetailDeferredRoute__output_type,
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
