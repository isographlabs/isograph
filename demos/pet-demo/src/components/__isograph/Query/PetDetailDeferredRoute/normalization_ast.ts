import type {NormalizationAst} from '@isograph/react';
const normalizationAst: NormalizationAst = {
  kind: "NormalizationAst",
  selections: [
    {
      kind: "Linked",
      isFallible: true,
      fieldName: "namable",
      arguments: null,
      concreteType: null,
      selections: [
        {
          kind: "Scalar",
          isFallible: false,
          fieldName: "__typename",
          arguments: null,
        },
      ],
    },
    {
      kind: "Linked",
      isFallible: true,
      fieldName: "notImplemented",
      arguments: null,
      concreteType: null,
      selections: [
        {
          kind: "Scalar",
          isFallible: false,
          fieldName: "__typename",
          arguments: null,
        },
      ],
    },
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
    {
      kind: "Linked",
      isFallible: true,
      fieldName: "topLevelField",
      arguments: [
        [
          "input",
          {
            kind: "Object",
            value: [
              [
                "name",
                { kind: "String", value: "ThisIsJustHereToTestObjectLiterals" },
              ],

            ]
          },
        ],
      ],
      concreteType: "TopLevelField",
      selections: [
        {
          kind: "Scalar",
          isFallible: false,
          fieldName: "__typename",
          arguments: null,
        },
      ],
    },
  ],
};
export default normalizationAst;
