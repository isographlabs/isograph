import type {IsographEntrypoint, NormalizationAst, RefetchQueryNormalizationArtifactWrapper} from '@isograph/react';
import {Pet__PetCheckinsCardList__param} from './param_type';
import {Pet__PetCheckinsCardList__output_type} from './output_type';
import readerResolver from './resolver_reader';
import normalizationAst from './normalization_ast';
import refetchQuery0 from './__refetch__0';
const nestedRefetchQueries: RefetchQueryNormalizationArtifactWrapper[] = [
  { artifact: refetchQuery0, allowedVariables: ["checkin_id", ] },
];

const artifact: IsographEntrypoint<
  Pet__PetCheckinsCardList__param,
  Pet__PetCheckinsCardList__output_type,
  NormalizationAst
> = {
  kind: "Entrypoint",
  networkRequestInfo: {
    kind: "NetworkRequestInfo",
    operation: {
      kind: "Operation",
      documentId: "2be4626786a6adb2b81cb6c29f0947f76ff415d27693ae0fcd6ac0f4c652c146",
      operationName: "PetCheckinsCardList",
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
