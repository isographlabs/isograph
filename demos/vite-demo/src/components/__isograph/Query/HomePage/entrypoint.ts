import type {IsographEntrypoint, NormalizationAst, RefetchQueryNormalizationArtifactWrapper} from '@isograph/react';
import {Query__HomePage__param} from './param_type';
import {Query__HomePage__output_type} from './output_type';
import readerResolver from './resolver_reader';
import queryText from './query_text';
const nestedRefetchQueries: RefetchQueryNormalizationArtifactWrapper[] = [];

const normalizationAst: NormalizationAst = {
  kind: "NormalizationAst",
  selections: [
    {
      kind: "Linked",
      fieldName: "getAllPokemon",
      arguments: [
        [
          "take",
          { kind: "Literal", value: 232 },
        ],

        [
          "offset",
          { kind: "Literal", value: 93 },
        ],
      ],
      concreteType: "Pokemon",
      selections: [
        {
          kind: "Scalar",
          fieldName: "bulbapediaPage",
          arguments: null,
        },
        {
          kind: "Scalar",
          fieldName: "forme",
          arguments: null,
        },
        {
          kind: "Scalar",
          fieldName: "key",
          arguments: null,
        },
        {
          kind: "Scalar",
          fieldName: "num",
          arguments: null,
        },
        {
          kind: "Scalar",
          fieldName: "species",
          arguments: null,
        },
        {
          kind: "Scalar",
          fieldName: "sprite",
          arguments: null,
        },
      ],
    },
  ],
};
const artifact: IsographEntrypoint<
  Query__HomePage__param,
  Query__HomePage__output_type,
  NormalizationAst
> = {
  kind: "Entrypoint",
  networkRequestInfo: {
    kind: "NetworkRequestInfo",
    queryText,
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
