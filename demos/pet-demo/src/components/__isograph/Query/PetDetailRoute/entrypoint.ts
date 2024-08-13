import type {IsographEntrypoint, NormalizationAst, RefetchQueryNormalizationArtifactWrapper} from '@isograph/react';
import {Query__PetDetailRoute__param} from './param_type';
import {Query__PetDetailRoute__output_type} from './output_type';
import readerResolver from './resolver_reader';
import refetchQuery0 from './__refetch__0';
import refetchQuery1 from './__refetch__1';
import refetchQuery2 from './__refetch__2';
import refetchQuery3 from './__refetch__3';
import refetchQuery4 from './__refetch__4';
const nestedRefetchQueries: RefetchQueryNormalizationArtifactWrapper[] = [
  { artifact: refetchQuery0, allowedVariables: ["id", ] },
  { artifact: refetchQuery1, allowedVariables: ["id", "new_best_friend_id", ] },
  { artifact: refetchQuery2, allowedVariables: ["input", ] },
  { artifact: refetchQuery3, allowedVariables: ["checkin_id", ] },
  { artifact: refetchQuery4, allowedVariables: ["id", ] },
];

const queryText = 'query PetDetailRoute ($id: ID!) {\
  pet____id___v_id: pet(id: $id) {\
    id,\
    age,\
    best_friend_relationship {\
      best_friend {\
        id,\
        name,\
        picture,\
      },\
      picture_together,\
    },\
    checkins____skip___l_null____limit___l_null: checkins(skip: null, limit: null) {\
      id,\
      location,\
      time,\
    },\
    favorite_phrase,\
    name,\
    nickname,\
    potential_new_best_friends {\
      id,\
      name,\
    },\
    stats {\
      cuteness,\
      energy,\
      hunger,\
      intelligence,\
      sociability,\
      weight,\
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
        kind: "Scalar",
        fieldName: "age",
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
        arguments: [
          [
            "skip",
            { kind: "Literal", value: null },
          ],

          [
            "limit",
            { kind: "Literal", value: null },
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
        kind: "Scalar",
        fieldName: "nickname",
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
        kind: "Linked",
        fieldName: "stats",
        arguments: null,
        selections: [
          {
            kind: "Scalar",
            fieldName: "cuteness",
            arguments: null,
          },
          {
            kind: "Scalar",
            fieldName: "energy",
            arguments: null,
          },
          {
            kind: "Scalar",
            fieldName: "hunger",
            arguments: null,
          },
          {
            kind: "Scalar",
            fieldName: "intelligence",
            arguments: null,
          },
          {
            kind: "Scalar",
            fieldName: "sociability",
            arguments: null,
          },
          {
            kind: "Scalar",
            fieldName: "weight",
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
const artifact: IsographEntrypoint<
  Query__PetDetailRoute__param,
  Query__PetDetailRoute__output_type
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
