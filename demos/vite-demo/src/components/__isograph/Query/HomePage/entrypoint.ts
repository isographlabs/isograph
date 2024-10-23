import type {IsographEntrypoint, NormalizationAst, RefetchQueryNormalizationArtifactWrapper} from '@isograph/react';
import {Query__HomePage__param} from './param_type';
import {Query__HomePage__output_type} from './output_type';
import readerResolver from './resolver_reader';
const nestedRefetchQueries: RefetchQueryNormalizationArtifactWrapper[] = [];

const queryText = 'query HomePage  {\
  getAllPokemon____take___l_232____offset___l_93: getAllPokemon(take: 232, offset: 93) {\
    bulbapediaPage,\
    forme,\
    key,\
    num,\
    species,\
    sprite,\
  },\
}';

const normalizationAst: NormalizationAst = [
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
];
const artifact: IsographEntrypoint<
  Query__HomePage__param,
  Query__HomePage__output_type
> = {
  kind: "Entrypoint",
  queryText,
  normalizationAst,
  readerWithRefetchQueries: {
    kind: "ReaderWithRefetchQueries",
    nestedRefetchQueries,
    readerArtifact: readerResolver,
  },
};

export default artifact;
