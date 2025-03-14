import type {NormalizationAst} from '@isograph/react';
const normalizationAst: NormalizationAst = {
  kind: "NormalizationAst",
  selections: [
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
          fieldName: "name",
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
