import type {IsographEntrypoint, NormalizationAst, RefetchQueryNormalizationArtifactWrapper} from '@isograph/react';
import {Query__PetDetailRoute__param} from './param_type';
import {Query__PetDetailRoute__output_type} from './output_type';
import readerResolver from './resolver_reader';
import queryText from './query_text';
import normalizationAst from './normalization_ast';
import refetchQuery0 from './__refetch__0';
import refetchQuery1 from './__refetch__1';
import refetchQuery2 from './__refetch__2';
import refetchQuery3 from './__refetch__3';
import refetchQuery4 from './__refetch__4';
import refetchQuery5 from './__refetch__5';
const nestedRefetchQueries: RefetchQueryNormalizationArtifactWrapper[] = [
  { artifact: refetchQuery0, allowedVariables: ["id", ] },
  { artifact: refetchQuery1, allowedVariables: ["id", ] },
  { artifact: refetchQuery2, allowedVariables: ["id", "new_best_friend_id", ] },
  { artifact: refetchQuery3, allowedVariables: ["input", ] },
  { artifact: refetchQuery4, allowedVariables: ["checkin_id", ] },
  { artifact: refetchQuery5, allowedVariables: ["id", ] },
];

const artifact: IsographEntrypoint<
  Query__PetDetailRoute__param,
  Query__PetDetailRoute__output_type,
  NormalizationAst
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
