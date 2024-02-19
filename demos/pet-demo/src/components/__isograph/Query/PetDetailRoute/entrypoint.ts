import type {IsographEntrypoint, NormalizationAst, RefetchQueryArtifactWrapper} from '@isograph/react';
import type {Query__PetDetailRoute__param, Query__PetDetailRoute__outputType} from './reader';
import readerResolver from './reader';
import refetchQuery0 from './__refetch__0';
import refetchQuery1 from './__refetch__1';
const nestedRefetchQueries: RefetchQueryArtifactWrapper[] = [{ artifact: refetchQuery0, allowedVariables: [] }, { artifact: refetchQuery1, allowedVariables: [] }, ];

const queryText = 'query PetDetailRoute ($id: ID!) {\
  pet____id___v_id: pet(id: $id) {\
    id,\
    best_friend_relationship {\
      best_friend {\
        id,\
        name,\
        picture,\
      },\
      picture_together,\
    },\
    checkins {\
      id,\
      location,\
      time,\
    },\
    favorite_phrase,\
    name,\
    potential_new_best_friends {\
      id,\
      name,\
    },\
    tagline,\
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
        kind: "Linked",
        fieldName: "best_friend_relationship",
        arguments: null,
        selections: [
          {
            kind: "Linked",
            fieldName: "best_friend",
            arguments: null,
            selections: [
              {
                kind: "Scalar",
                fieldName: "id",
                arguments: null,
              },
              {
                kind: "Scalar",
                fieldName: "name",
                arguments: null,
              },
              {
                kind: "Scalar",
                fieldName: "picture",
                arguments: null,
              },
            ],
          },
          {
            kind: "Scalar",
            fieldName: "picture_together",
            arguments: null,
          },
        ],
      },
      {
        kind: "Linked",
        fieldName: "checkins",
        arguments: null,
        selections: [
          {
            kind: "Scalar",
            fieldName: "id",
            arguments: null,
          },
          {
            kind: "Scalar",
            fieldName: "location",
            arguments: null,
          },
          {
            kind: "Scalar",
            fieldName: "time",
            arguments: null,
          },
        ],
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
      {
        kind: "Linked",
        fieldName: "potential_new_best_friends",
        arguments: null,
        selections: [
          {
            kind: "Scalar",
            fieldName: "id",
            arguments: null,
          },
          {
            kind: "Scalar",
            fieldName: "name",
            arguments: null,
          },
        ],
      },
      {
        kind: "Scalar",
        fieldName: "tagline",
        arguments: null,
      },
    ],
  },
];
const artifact: IsographEntrypoint<Query__PetDetailRoute__param,
  Query__PetDetailRoute__param,
  Query__PetDetailRoute__outputType
> = {
  kind: "Entrypoint",
  queryText,
  normalizationAst,
  nestedRefetchQueries,
  readerArtifact: readerResolver,
};

export default artifact;
