import type {NormalizationAst} from '@isograph/react';
const normalizationAst: NormalizationAst = {
  kind: "NormalizationAst",
  selections: [
    {
      kind: "Linked",
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
};
export default normalizationAst;
