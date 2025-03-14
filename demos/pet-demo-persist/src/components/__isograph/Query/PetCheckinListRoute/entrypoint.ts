import type {IsographEntrypoint, NormalizationAst, RefetchQueryNormalizationArtifactWrapper} from '@isograph/react';
import {Query__PetCheckinListRoute__param} from './param_type';
import {Query__PetCheckinListRoute__output_type} from './output_type';
import readerResolver from './resolver_reader';
import normalizationAst from './normalization_ast';
import refetchQuery0 from './__refetch__0';
const nestedRefetchQueries: RefetchQueryNormalizationArtifactWrapper[] = [
  { artifact: refetchQuery0, allowedVariables: ["checkin_id", ] },
];

const artifact: IsographEntrypoint<
  Query__PetCheckinListRoute__param,
  Query__PetCheckinListRoute__output_type,
  NormalizationAst
> = {
  kind: "Entrypoint",
  networkRequestInfo: {
    kind: "NetworkRequestInfo",
    operation: {
      kind: "Operation",
      documentId: "4e9af6becccbd3ee729fb1bb4187742db64a7092469bf2b79286687d7ec7b94e",
      operationName: "PetCheckinListRoute",
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
