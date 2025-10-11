import type {IsographEntrypoint, NormalizationAst, RefetchQueryNormalizationArtifactWrapper} from '@isograph/react';
import {Query__SmartestPetRoute__param} from './param_type';
import {Query__SmartestPetRoute__output_type} from './output_type';
import readerResolver from './resolver_reader';
import queryText from './query_text';
import normalizationAst from './normalization_ast';
import refetchQuery0 from './__refetch__0';
import refetchQuery1 from './__refetch__1';
const nestedRefetchQueries: RefetchQueryNormalizationArtifactWrapper[] = [
  { artifact: refetchQuery0, allowedVariables: ["id", ] },
  { artifact: refetchQuery1, allowedVariables: ["id", ] },
];

const artifact: IsographEntrypoint<
  Query__SmartestPetRoute__param,
  Query__SmartestPetRoute__output_type,
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
