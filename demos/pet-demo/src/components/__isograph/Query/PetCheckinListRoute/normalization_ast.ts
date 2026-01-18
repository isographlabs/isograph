import type {NormalizationAst} from '@isograph/react';
const normalizationAst: NormalizationAst = {
  kind: "NormalizationAst",
  selections: [
    {
      kind: "Linked",
      isFallible: true,
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
          isFallible: false,
          fieldName: "id",
          arguments: null,
        },
        {
          kind: "Linked",
          isFallible: false,
          fieldName: "checkins",
          arguments: [
            [
              "skip",
              { kind: "Literal", value: 0 },
            ],

            [
              "limit",
              { kind: "Literal", value: 1 },
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
};
export default normalizationAst;
