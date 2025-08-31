import type {NormalizationAst} from '@isograph/react';
const normalizationAst: NormalizationAst = {
  kind: "NormalizationAst",
  selections: [
    {
      kind: "Linked",
      fieldName: "pets",
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
          fieldName: "stats",
          arguments: null,
          concreteType: "PetStats",
          selections: [
            {
              kind: "Scalar",
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
