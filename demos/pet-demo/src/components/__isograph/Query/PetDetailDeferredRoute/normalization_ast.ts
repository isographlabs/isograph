import type {NormalizationAst} from '@isograph/react';
const normalizationAst: NormalizationAst = {
  kind: "NormalizationAst",
  selections: [
    {
      kind: "Linked",
      fieldName: "namable",
      arguments: null,
      concreteType: null,
      selections: [
        {
          kind: "Scalar",
          fieldName: "__typename",
          arguments: null,
        },
      ],
    },
    {
      kind: "Linked",
      fieldName: "notImplemented",
      arguments: null,
      concreteType: null,
      selections: [
        {
          kind: "Scalar",
          fieldName: "__typename",
          arguments: null,
        },
      ],
    },
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
    {
      kind: "Linked",
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
          fieldName: "__typename",
          arguments: null,
        },
      ],
    },
  ],
};
export default normalizationAst;
