import type {IsographEntrypoint, NormalizationAst, RefetchQueryNormalizationArtifactWrapper} from '@isograph/react';
import {User__RepositoryConnection__param} from './param_type';
import {User__RepositoryConnection__output_type} from './output_type';
import type {User__RepositoryConnection__raw_response_type} from './raw_response_type';
import readerResolver from './resolver_reader';
import queryText from './query_text';
import normalizationAst from './normalization_ast';
const nestedRefetchQueries: RefetchQueryNormalizationArtifactWrapper[] = [];

const artifact: IsographEntrypoint<
  User__RepositoryConnection__param,
  User__RepositoryConnection__output_type,
  NormalizationAst,
  User__RepositoryConnection__raw_response_type
> = {
  kind: "Entrypoint",
  networkRequestInfo: {
    kind: "NetworkRequestInfo",
    operation: {
      kind: "Operation",
      text: queryText,
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
