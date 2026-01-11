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
      ],
    },
  ],
};
export default normalizationAst;
