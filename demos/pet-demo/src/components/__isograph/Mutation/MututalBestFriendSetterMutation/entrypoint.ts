import type {IsographEntrypoint, NormalizationAst, RefetchQueryNormalizationArtifactWrapper} from '@isograph/react';
import {Mutation__MututalBestFriendSetterMutation__param} from './param_type';
import {Mutation__MututalBestFriendSetterMutation__output_type} from './output_type';
import type {Mutation__MututalBestFriendSetterMutation__raw_response_type} from './raw_response_type';
import readerResolver from './resolver_reader';
import queryText from './query_text';
import normalizationAst from './normalization_ast';
const nestedRefetchQueries: RefetchQueryNormalizationArtifactWrapper[] = [];

const artifact: IsographEntrypoint<
  Mutation__MututalBestFriendSetterMutation__param,
  Mutation__MututalBestFriendSetterMutation__output_type,
  NormalizationAst,
  Mutation__MututalBestFriendSetterMutation__raw_response_type
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
  concreteType: "Mutation",
  readerWithRefetchQueries: {
    kind: "ReaderWithRefetchQueries",
    nestedRefetchQueries,
    readerArtifact: readerResolver,
  },
};

export default artifact;
