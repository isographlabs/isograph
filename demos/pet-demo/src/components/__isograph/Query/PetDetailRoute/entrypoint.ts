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

const normalizationAst: NormalizationAst = {
  kind: "NormalizationAst",
  selections: [
    {
      kind: "Linked",
      fieldName: "pet",
      arguments: [
        [
          "id",
          { kind: "Variable", name: "id" },
        ],
      ],
      concreteType: "Pet",
      selections: [
        {
          kind: "Scalar",
          fieldName: "id",
          arguments: null,
          isUpdatable: false,
        },
        {
          kind: "Scalar",
          fieldName: "age",
          arguments: null,
          isUpdatable: false,
        },
        {
          kind: "Linked",
          fieldName: "best_friend_relationship",
          arguments: null,
          concreteType: "BestFriendRelationship",
          selections: [
            {
              kind: "Linked",
              fieldName: "best_friend",
              arguments: null,
              concreteType: "Pet",
              selections: [
                {
                  kind: "Scalar",
                  fieldName: "id",
                  arguments: null,
                  isUpdatable: false,
                },
                {
                  kind: "Scalar",
                  fieldName: "name",
                  arguments: null,
                  isUpdatable: false,
                },
                {
                  kind: "Scalar",
                  fieldName: "picture",
                  arguments: null,
                  isUpdatable: false,
                },
              ],
            },
            {
              kind: "Scalar",
              fieldName: "picture_together",
              arguments: null,
              isUpdatable: false,
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
          concreteType: "Checkin",
          selections: [
            {
              kind: "Scalar",
              fieldName: "id",
              arguments: null,
              isUpdatable: false,
            },
            {
              kind: "Scalar",
              fieldName: "location",
              arguments: null,
              isUpdatable: false,
            },
            {
              kind: "Scalar",
              fieldName: "time",
              arguments: null,
              isUpdatable: false,
            },
          ],
        },
        {
          kind: "Scalar",
          fieldName: "favorite_phrase",
          arguments: null,
          isUpdatable: false,
        },
        {
          kind: "Scalar",
          fieldName: "name",
          arguments: null,
          isUpdatable: false,
        },
        {
          kind: "Scalar",
          fieldName: "nickname",
          arguments: null,
          isUpdatable: false,
        },
        {
          kind: "Linked",
          fieldName: "potential_new_best_friends",
          arguments: null,
          concreteType: "Pet",
          selections: [
            {
              kind: "Scalar",
              fieldName: "id",
              arguments: null,
              isUpdatable: false,
            },
            {
              kind: "Scalar",
              fieldName: "name",
              arguments: null,
              isUpdatable: false,
            },
          ],
        },
        {
          kind: "Linked",
          fieldName: "stats",
          arguments: null,
          concreteType: "PetStats",
          selections: [
            {
              kind: "Scalar",
              fieldName: "cuteness",
              arguments: null,
              isUpdatable: false,
            },
            {
              kind: "Scalar",
              fieldName: "energy",
              arguments: null,
              isUpdatable: false,
            },
            {
              kind: "Scalar",
              fieldName: "hunger",
              arguments: null,
              isUpdatable: false,
            },
            {
              kind: "Scalar",
              fieldName: "intelligence",
              arguments: null,
              isUpdatable: false,
            },
            {
              kind: "Scalar",
              fieldName: "sociability",
              arguments: null,
              isUpdatable: false,
            },
            {
              kind: "Scalar",
              fieldName: "weight",
              arguments: null,
              isUpdatable: false,
            },
          ],
        },
        {
          kind: "Scalar",
          fieldName: "tagline",
          arguments: null,
          isUpdatable: true,
        },
      ],
    },
  ],
};
const artifact: IsographEntrypoint<
  Query__PetDetailRoute__param,
  Query__PetDetailRoute__output_type,
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
