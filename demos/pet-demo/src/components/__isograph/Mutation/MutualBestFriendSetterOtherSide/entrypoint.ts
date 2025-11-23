import type {IsographEntrypoint, NormalizationAst, RefetchQueryNormalizationArtifactWrapper} from '@isograph/react';
import {Mutation__MutualBestFriendSetterOtherSide__param} from './param_type';
import {Mutation__MutualBestFriendSetterOtherSide__output_type} from './output_type';
import {Mutation__MutualBestFriendSetterOtherSide__raw_response_type} from './raw_response_type';
import readerResolver from './resolver_reader';
import queryText from './query_text';
import normalizationAst from './normalization_ast';
const nestedRefetchQueries: RefetchQueryNormalizationArtifactWrapper[] = [];

const artifact: IsographEntrypoint<
  Mutation__MutualBestFriendSetterOtherSide__param,
  Mutation__MutualBestFriendSetterOtherSide__output_type,
  NormalizationAst,
  Mutation__MutualBestFriendSetterOtherSide__raw_response_type
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
