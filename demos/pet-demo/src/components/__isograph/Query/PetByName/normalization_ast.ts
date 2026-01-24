import type {NormalizationAst} from '@isograph/react';
const normalizationAst: NormalizationAst = {
  kind: "NormalizationAst",
  selections: [
    {
      kind: "Linked",
      isFallible: true,
      fieldName: "petByName",
      arguments: [
        [
          "name",
          { kind: "Variable", name: "name" },
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
