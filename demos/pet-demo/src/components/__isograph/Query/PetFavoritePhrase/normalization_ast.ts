import type {NormalizationAst} from '@isograph/react';
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
          fieldName: "favorite_phrase",
          arguments: null,
        },
        {
          kind: "Scalar",
          fieldName: "name",
          arguments: null,
        },
      ],
    },
  ],
};
export default normalizationAst;
