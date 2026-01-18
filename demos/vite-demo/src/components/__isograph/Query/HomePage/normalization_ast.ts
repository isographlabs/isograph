import type {NormalizationAst} from '@isograph/react';
const normalizationAst: NormalizationAst = {
  kind: "NormalizationAst",
  selections: [
    {
      kind: "Linked",
      isFallible: false,
      fieldName: "getAllPokemon",
      arguments: [
        [
          "take",
          { kind: "Literal", value: 232 },
        ],

        [
          "offset",
          { kind: "Literal", value: 93 },
        ],
      ],
      concreteType: "Pokemon",
      selections: [
        {
          kind: "Scalar",
          isFallible: false,
          fieldName: "bulbapediaPage",
          arguments: null,
        },
        {
          kind: "Scalar",
          isFallible: true,
          fieldName: "forme",
          arguments: null,
        },
        {
          kind: "Scalar",
          isFallible: false,
          fieldName: "key",
          arguments: null,
        },
        {
          kind: "Scalar",
          isFallible: false,
          fieldName: "num",
          arguments: null,
        },
        {
          kind: "Scalar",
          isFallible: false,
          fieldName: "species",
          arguments: null,
        },
        {
          kind: "Scalar",
          isFallible: false,
          fieldName: "sprite",
          arguments: null,
        },
      ],
    },
  ],
};
export default normalizationAst;
