import type {NormalizationAst} from '@isograph/react';
const normalizationAst: NormalizationAst = {
  kind: "NormalizationAst",
  selections: [
    {
      kind: "Linked",
      isFallible: false,
      fieldName: "set_pet_best_friend",
      arguments: [
        [
          "id",
          { kind: "Variable", name: "pet_id" },
        ],

        [
          "new_best_friend_id",
          { kind: "Variable", name: "new_best_friend_id" },
        ],
      ],
      concreteType: "SetBestFriendResponse",
      selections: [
        {
          kind: "Linked",
          isFallible: false,
          fieldName: "pet",
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
                  ],
                },
              ],
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
      ],
    },
  ],
};
export default normalizationAst;
