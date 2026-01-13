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
          kind: "Linked",
          isFallible: true,
          fieldName: "stats",
          arguments: null,
          concreteType: "PetStats",
          selections: [
            {
              kind: "Scalar",
              isFallible: true,
              fieldName: "intelligence",
              arguments: null,
            },
          ],
        },
      ],
    },
  ],
};
export default normalizationAst;
