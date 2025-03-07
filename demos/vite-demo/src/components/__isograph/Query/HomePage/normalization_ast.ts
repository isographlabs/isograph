import type {NormalizationAst} from '@isograph/react';
const normalizationAst: NormalizationAst = {
  kind: "NormalizationAst",
  selections: [
    {
      kind: "Linked",
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
          fieldName: "bulbapediaPage",
          arguments: null,
        },
        {
          kind: "Scalar",
          fieldName: "forme",
          arguments: null,
        },
        {
          kind: "Scalar",
          fieldName: "key",
          arguments: null,
        },
        {
          kind: "Scalar",
          fieldName: "num",
          arguments: null,
        },
        {
          kind: "Scalar",
          fieldName: "species",
          arguments: null,
        },
        {
          kind: "Scalar",
          fieldName: "sprite",
          arguments: null,
        },
      ],
    },
  ],
};
export default normalizationAst;
