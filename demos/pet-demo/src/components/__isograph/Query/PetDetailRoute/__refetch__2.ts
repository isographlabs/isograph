import type { IsographEntrypoint, ReaderAst, FragmentReference, NormalizationAst, RefetchQueryNormalizationArtifact } from '@isograph/react';
const queryText = 'mutation Query__set_pet_tagline ($input: SetPetTaglineParams!) {\
  set_pet_tagline____input___v_input: set_pet_tagline(input: $input) {\
    pet {\
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
  },\
}';

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
    },
  ],
};
const artifact: RefetchQueryNormalizationArtifact = {
  kind: "RefetchQuery",
  networkRequestInfo: {
    kind: "NetworkRequestInfo",
    queryText,
    normalizationAst,
  },
  concreteType: "Mutation",
};

export default artifact;
