import type {IsographEntrypoint, NormalizationAst, RefetchQueryNormalizationArtifactWrapper} from '@isograph/react';
import {AdItem__AdItemDisplay__param} from './param_type';
import {AdItem__AdItemDisplay__output_type} from './output_type';
import readerResolver from './resolver_reader';
import normalizationAst from './normalization_ast';
const nestedRefetchQueries: RefetchQueryNormalizationArtifactWrapper[] = [];

const artifact: IsographEntrypoint<
  AdItem__AdItemDisplay__param,
  AdItem__AdItemDisplay__output_type,
  NormalizationAst
> = {
  kind: "Entrypoint",
  networkRequestInfo: {
    kind: "NetworkRequestInfo",
    operation: {
      kind: "Operation",
      documentId: "54b26723f0ea0ea4c07b1e76e5210e377857e9e06ed75d66a90ae4bc47c84592",
      operationName: "AdItemDisplay",
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
