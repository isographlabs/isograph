import type {IsographEntrypoint, NormalizationAst, RefetchQueryNormalizationArtifactWrapper} from '@isograph/react';
import {Viewer__NewsfeedPaginationComponent__param} from './param_type';
import {Viewer__NewsfeedPaginationComponent__output_type} from './output_type';
import readerResolver from './resolver_reader';
import normalizationAst from './normalization_ast';
const nestedRefetchQueries: RefetchQueryNormalizationArtifactWrapper[] = [];

const artifact: IsographEntrypoint<
  Viewer__NewsfeedPaginationComponent__param,
  Viewer__NewsfeedPaginationComponent__output_type,
  NormalizationAst
> = {
  kind: "Entrypoint",
  networkRequestInfo: {
    kind: "NetworkRequestInfo",
    operation: {
      kind: "Operation",
      documentId: "952f3ce0b338cd1c2909e139c75716daa9cf10da55f34a71108662c53d299a16",
      operationName: "NewsfeedPaginationComponent",
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
