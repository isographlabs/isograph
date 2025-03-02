import type {IsographEntrypoint, NormalizationAst, RefetchQueryNormalizationArtifactWrapper} from '@isograph/react';
import {Query__PetDetailRoute__param} from './param_type';
import {Query__PetDetailRoute__output_type} from './output_type';
import readerResolver from './resolver_reader';
import queryText from './query_text';
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
          concreteType: "Checkin",
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
          concreteType: "Pet",
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
          concreteType: "PetStats",
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
