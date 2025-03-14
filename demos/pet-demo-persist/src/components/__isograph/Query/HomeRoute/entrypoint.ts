import type {IsographEntrypoint, NormalizationAst, RefetchQueryNormalizationArtifactWrapper} from '@isograph/react';
import {Query__HomeRoute__param} from './param_type';
import {Query__HomeRoute__output_type} from './output_type';
import readerResolver from './resolver_reader';
import normalizationAst from './normalization_ast';
const nestedRefetchQueries: RefetchQueryNormalizationArtifactWrapper[] = [];

const artifact: IsographEntrypoint<
  Query__HomeRoute__param,
  Query__HomeRoute__output_type,
  NormalizationAst
> = {
  kind: "Entrypoint",
  networkRequestInfo: {
    kind: "NetworkRequestInfo",
    operation: {
      kind: "Operation",
      documentId: "bc81a81318ea8063d78aafd75ae1fcf7229ca73ed0efe09cdfd24dde5452dd87",
      operationName: "HomeRoute",
      operationKind: "Query",
      text: null,
    },
    normalizationAst,
  },
  concreteType: "Query",
  readerWithRefetchQueries: {
    kind: "ReaderWithRefetchQueries",
    nestedRefetchQueries,
    readerArtifact: readerResolver,
  },
};

export default artifact;
