import type {IsographEntrypoint, NormalizationAst, RefetchQueryArtifactWrapper} from '@isograph/react';
import {Query__PetFavoritePhrase__param} from './param_type';
import {Query__PetFavoritePhrase__outputType} from './output_type';
import readerResolver from './reader';
const nestedRefetchQueries: RefetchQueryArtifactWrapper[] = [];

const queryText = 'query PetFavoritePhrase ($id: ID!) {\
  pet____id___v_id: pet(id: $id) {\
    id,\
    favorite_phrase,\
    name,\
  },\
}';

const normalizationAst: NormalizationAst = [
  {
    kind: "Linked",
    fieldName: "pet",
    arguments: [
      [
        "id",
        { kind: "Variable", name: "id" },
      ],
    ],
    selections: [
      {
        kind: "Scalar",
        fieldName: "id",
        arguments: null,
      },
      {
        kind: "Scalar",
        fieldName: "favorite_phrase",
        arguments: null,
      },
      {
        kind: "Scalar",
        fieldName: "name",
        arguments: null,
      },
    ],
  },
];
const artifact: IsographEntrypoint<
  Query__PetFavoritePhrase__param,
  Query__PetFavoritePhrase__outputType
> = {
  kind: "Entrypoint",
  queryText,
  normalizationAst,
  nestedRefetchQueries,
  readerArtifact: readerResolver,
};

export default artifact;
