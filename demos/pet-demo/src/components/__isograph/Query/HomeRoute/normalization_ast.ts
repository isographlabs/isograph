import type {NormalizationAst} from '@isograph/react';
const normalizationAst: NormalizationAst = {
  kind: "NormalizationAst",
  selections: [
    {
      kind: "Linked",
      isFallible: false,
      fieldName: "pets",
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
        {
          kind: "Scalar",
          isFallible: false,
          fieldName: "picture",
          arguments: null,
        },
        {
          kind: "Scalar",
          isFallible: false,
          fieldName: "tagline",
          arguments: null,
        },
      ],
    },
  ],
};
export default normalizationAst;
