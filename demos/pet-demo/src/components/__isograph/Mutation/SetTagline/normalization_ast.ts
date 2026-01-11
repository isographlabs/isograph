import type {NormalizationAst} from '@isograph/react';
const normalizationAst: NormalizationAst = {
  kind: "NormalizationAst",
  selections: [
    {
      kind: "Linked",
      isFallible: false,
      fieldName: "set_pet_tagline",
      arguments: [
        [
          "input",
          { kind: "Variable", name: "input" },
        ],
      ],
      concreteType: "SetPetTaglineResponse",
      selections: [
        {
          kind: "Linked",
          isFallible: false,
          fieldName: "pet",
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
              fieldName: "tagline",
              arguments: null,
            },
          ],
        },
      ],
    },
  ],
};
export default normalizationAst;
