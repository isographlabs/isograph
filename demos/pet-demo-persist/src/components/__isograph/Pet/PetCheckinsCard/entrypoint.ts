import type {IsographEntrypoint, NormalizationAst, RefetchQueryNormalizationArtifactWrapper} from '@isograph/react';
import {Pet__PetCheckinsCard__param} from './param_type';
import {Pet__PetCheckinsCard__output_type} from './output_type';
import readerResolver from './resolver_reader';
import normalizationAst from './normalization_ast';
import refetchQuery0 from './__refetch__0';
const nestedRefetchQueries: RefetchQueryNormalizationArtifactWrapper[] = [
  { artifact: refetchQuery0, allowedVariables: ["checkin_id", ] },
];

const artifact: IsographEntrypoint<
  Pet__PetCheckinsCard__param,
  Pet__PetCheckinsCard__output_type,
  NormalizationAst
> = {
  kind: "Entrypoint",
  networkRequestInfo: {
    kind: "NetworkRequestInfo",
    operation: {
      kind: "Operation",
      documentId: "84f5b06f563d573dfb1a7fe7f1145100b3f928eb1b99f46a4a6d1eb66fa34662",
      operationName: "PetCheckinsCard",
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
