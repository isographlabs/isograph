import type {NormalizationAst} from '@isograph/react';
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
          { kind: "Literal", value: 0 },
        ],
      ],
      concreteType: null,
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
          kind: "InlineFragment",
          type: "Economist",
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
              fieldName: "name",
              arguments: null,
            },
          ],
        },
      ],
    },
    {
      kind: "Linked",
      isFallible: true,
      fieldName: "node",
      arguments: [
        [
          "id",
          { kind: "Literal", value: 1 },
        ],
      ],
      concreteType: null,
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
          kind: "InlineFragment",
          type: "Economist",
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
              fieldName: "name",
              arguments: null,
            },
          ],
        },
      ],
    },
  ],
};
export default normalizationAst;
