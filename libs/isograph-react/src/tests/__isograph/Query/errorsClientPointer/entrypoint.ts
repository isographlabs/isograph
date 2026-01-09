import type {IsographEntrypoint, NormalizationAst, RefetchQueryNormalizationArtifactWrapper} from '@isograph/react';
import {Query__errorsClientPointer__param} from './param_type';
import {Query__errorsClientPointer__output_type} from './output_type';
import type {Query__errorsClientPointer__raw_response_type} from './raw_response_type';
import readerResolver from './resolver_reader';
import queryText from './query_text';
import normalizationAst from './normalization_ast';
import refetchQuery0 from './__refetch__0';
const nestedRefetchQueries: RefetchQueryNormalizationArtifactWrapper[] = [
  { artifact: refetchQuery0, allowedVariables: ["id", ] },
];

const artifact: IsographEntrypoint<
  Query__errorsClientPointer__param,
  Query__errorsClientPointer__output_type,
  NormalizationAst,
  Query__errorsClientPointer__raw_response_type
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
