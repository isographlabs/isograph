import type {NormalizationAst} from '@isograph/react';
const normalizationAst: NormalizationAst = {
  kind: "NormalizationAst",
  selections: [
    {
      kind: "Linked",
      fieldName: "node",
      arguments: [
        [
          "id",
          { kind: "Variable", name: "id" },
        ],
      ],
      concreteType: null,
      selections: [
        {
          kind: "InlineFragment",
          type: "Pet",
          selections: [
            {
              kind: "Scalar",
              fieldName: "__typename",
              arguments: null,
            },
            {
              kind: "Scalar",
              fieldName: "id",
              arguments: null,
            },
            {
              kind: "Linked",
              fieldName: "checkins",
              arguments: [
                [
                  "skip",
                  { kind: "Variable", name: "skip" },
                ],

                [
                  "limit",
                  { kind: "Variable", name: "limit" },
                ],
              ],
              concreteType: "Checkin",
              selections: [
                {
                  kind: "Scalar",
                  fieldName: "id",
                  arguments: null,
                },
                {
                  kind: "Scalar",
                  fieldName: "location",
                  arguments: null,
                },
                {
                  kind: "Scalar",
                  fieldName: "time",
                  arguments: null,
                },
              ],
            },
          ],
        },
      ],
    },
  ],
};
export default normalizationAst;
