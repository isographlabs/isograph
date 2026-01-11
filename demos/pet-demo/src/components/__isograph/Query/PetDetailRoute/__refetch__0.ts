import type { IsographEntrypoint, ReaderAst, FragmentReference, NormalizationAst, RefetchQueryNormalizationArtifact } from '@isograph/react';
import queryText from './__refetch__query_text__0';

const normalizationAst: NormalizationAst = {
  kind: "NormalizationAst",
  selections: [
    {
      kind: "Linked",
      isFallible: true,
      fieldName: "node",
      arguments: [
        [
          "id",
          { kind: "Variable", name: "id" },
        ],
      ],
      concreteType: null,
      selections: [
        {
          kind: "InlineFragment",
          type: "Pet",
          selections: [
            {
              kind: "Scalar",
              isFallible: false,
              fieldName: "__typename",
              arguments: null,
            },
            {
              kind: "Scalar",
              isFallible: false,
              fieldName: "id",
              arguments: null,
            },
            {
              kind: "Scalar",
              isFallible: false,
              fieldName: "age",
              arguments: null,
            },
            {
              kind: "Linked",
              isFallible: true,
              fieldName: "best_friend_relationship",
              arguments: null,
              concreteType: "BestFriendRelationship",
              selections: [
                {
                  kind: "Linked",
                  isFallible: false,
                  fieldName: "best_friend",
                  arguments: null,
                  concreteType: "Pet",
                  selections: [
                    {
                      kind: "Scalar",
                      isFallible: false,
                      fieldName: "id",
                      arguments: null,
                    },
                    {
                      kind: "Scalar",
                      isFallible: false,
                      fieldName: "firstName",
                      arguments: null,
                    },
                    {
                      kind: "Scalar",
                      isFallible: false,
                      fieldName: "lastName",
                      arguments: null,
                    },
                    {
                      kind: "Scalar",
                      isFallible: false,
                      fieldName: "picture",
                      arguments: null,
                    },
                  ],
                },
                {
                  kind: "Scalar",
                  isFallible: true,
                  fieldName: "picture_together",
                  arguments: null,
                },
              ],
            },
            {
              kind: "Linked",
              isFallible: false,
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
                  isFallible: false,
                  fieldName: "id",
                  arguments: null,
                },
                {
                  kind: "Scalar",
                  isFallible: false,
                  fieldName: "location",
                  arguments: null,
                },
                {
                  kind: "Scalar",
                  isFallible: false,
                  fieldName: "time",
                  arguments: null,
                },
              ],
            },
            {
              kind: "Scalar",
              isFallible: true,
              fieldName: "favorite_phrase",
              arguments: null,
            },
            {
              kind: "Scalar",
              isFallible: false,
              fieldName: "firstName",
              arguments: null,
            },
            {
              kind: "Scalar",
              isFallible: false,
              fieldName: "lastName",
              arguments: null,
            },
            {
              kind: "Scalar",
              isFallible: true,
              fieldName: "nickname",
              arguments: null,
            },
            {
              kind: "Linked",
              isFallible: false,
              fieldName: "potential_new_best_friends",
              arguments: null,
              concreteType: "Pet",
              selections: [
                {
                  kind: "Scalar",
                  isFallible: false,
                  fieldName: "id",
                  arguments: null,
                },
                {
                  kind: "Scalar",
                  isFallible: false,
                  fieldName: "firstName",
                  arguments: null,
                },
                {
                  kind: "Scalar",
                  isFallible: false,
                  fieldName: "lastName",
                  arguments: null,
                },
              ],
            },
            {
              kind: "Linked",
              isFallible: true,
              fieldName: "stats",
              arguments: null,
              concreteType: "PetStats",
              selections: [
                {
                  kind: "Scalar",
                  isFallible: true,
                  fieldName: "cuteness",
                  arguments: null,
                },
                {
                  kind: "Scalar",
                  isFallible: true,
                  fieldName: "energy",
                  arguments: null,
                },
                {
                  kind: "Scalar",
                  isFallible: true,
                  fieldName: "hunger",
                  arguments: null,
                },
                {
                  kind: "Scalar",
                  isFallible: true,
                  fieldName: "intelligence",
                  arguments: null,
                },
                {
                  kind: "Scalar",
                  isFallible: true,
                  fieldName: "sociability",
                  arguments: null,
                },
                {
                  kind: "Scalar",
                  isFallible: true,
                  fieldName: "weight",
                  arguments: null,
                },
              ],
            },
            {
              kind: "Scalar",
              isFallible: false,
              fieldName: "tagline",
              arguments: null,
            },
          ],
        },
      ],
    },
  ],
};
const artifact: RefetchQueryNormalizationArtifact = {
  kind: "RefetchQuery",
  networkRequestInfo: {
    kind: "NetworkRequestInfo",
    operation: {
      kind: "Operation",
      text: queryText,
    },
    normalizationAst,
  },
  concreteType: "Query",
};

export default artifact;
