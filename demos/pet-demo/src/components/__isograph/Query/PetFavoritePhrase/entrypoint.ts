import type {IsographEntrypoint, NormalizationAstLoader, RefetchQueryNormalizationArtifactWrapper} from '@isograph/react';
import {Query__PetFavoritePhrase__param} from './param_type';
import {Query__PetFavoritePhrase__output_type} from './output_type';
import type {Query__PetFavoritePhrase__raw_response_type} from './raw_response_type';
import queryText from './query_text';
const nestedRefetchQueries: RefetchQueryNormalizationArtifactWrapper[] = [];

const artifact: IsographEntrypoint<
  Query__PetFavoritePhrase__param,
  Query__PetFavoritePhrase__output_type,
  NormalizationAstLoader,
  Query__PetFavoritePhrase__raw_response_type
> = {
  kind: "Entrypoint",
  networkRequestInfo: {
    kind: "NetworkRequestInfo",
    operation: {
      kind: "Operation",
      text: queryText,
    },
    normalizationAst: {
      kind: "NormalizationAstLoader",
      loader: () => import('./normalization_ast').then(module => module.default),
    },
  },
  concreteType: "Query",
  readerWithRefetchQueries: {
    kind: "ReaderWithRefetchQueriesLoader",
    fieldName: "PetFavoritePhrase",
    loader: () => import('./resolver_reader')
      .then(module => ({
        kind: "ReaderWithRefetchQueries",
        nestedRefetchQueries,
        readerArtifact: module.default,
      }))
  }
};

export default artifact;
