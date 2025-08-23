import type { IsographEntrypoint, ReaderAst, FragmentReference, NormalizationAst, RefetchQueryNormalizationArtifact } from '@isograph/react';
import queryText from './__refetch__query_text__3';

const normalizationAst: NormalizationAst = {
  kind: "NormalizationAst",
  selections: [
    {
      kind: "Linked",
      fieldName: "set_pet_tagline",
      arguments: [
        [
          "input",
          { kind: "Variable", name: "input" },
        ],
      ],
      concreteType: "SetPetTaglineResponse",
      selections: [
        {
          kind: "Linked",
          fieldName: "pet",
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
  concreteType: "Mutation",
};

export default artifact;
