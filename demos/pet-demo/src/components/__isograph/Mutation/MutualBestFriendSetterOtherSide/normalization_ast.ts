import type {NormalizationAst} from '@isograph/react';
const normalizationAst: NormalizationAst = {
  kind: "NormalizationAst",
  selections: [
    {
      kind: "Linked",
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
                      fieldName: "firstName",
                      arguments: null,
                    },
                    {
                      kind: "Scalar",
                      fieldName: "lastName",
                      arguments: null,
                    },
                  ],
                },
              ],
            },
            {
              kind: "Scalar",
              fieldName: "firstName",
              arguments: null,
            },
            {
              kind: "Scalar",
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
