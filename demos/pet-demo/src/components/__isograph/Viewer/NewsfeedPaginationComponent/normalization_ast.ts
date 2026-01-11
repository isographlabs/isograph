import type {NormalizationAst} from '@isograph/react';
const normalizationAst: NormalizationAst = {
  kind: "NormalizationAst",
  selections: [
    {
      kind: "Linked",
      isFallible: true,
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
          type: "Viewer",
          selections: [
            {
              kind: "Scalar",
              isFallible: false,
              fieldName: "__typename",
              arguments: null,
            },
            {
              kind: "Scalar",
              isFallible: false,
              fieldName: "id",
              arguments: null,
            },
            {
              kind: "Linked",
              isFallible: false,
              fieldName: "newsfeed",
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
              concreteType: null,
              selections: [
                {
                  kind: "Scalar",
                  isFallible: false,
                  fieldName: "__typename",
                  arguments: null,
                },
                {
                  kind: "InlineFragment",
                  type: "AdItem",
                  selections: [
                    {
                      kind: "Scalar",
                      isFallible: false,
                      fieldName: "__typename",
                      arguments: null,
                    },
                    {
                      kind: "Scalar",
                      isFallible: false,
                      fieldName: "id",
                      arguments: null,
                    },
                  ],
                },
                {
                  kind: "InlineFragment",
                  type: "BlogItem",
                  selections: [
                    {
                      kind: "Scalar",
                      isFallible: false,
                      fieldName: "__typename",
                      arguments: null,
                    },
                    {
                      kind: "Scalar",
                      isFallible: false,
                      fieldName: "id",
                      arguments: null,
                    },
                    {
                      kind: "Scalar",
                      isFallible: false,
                      fieldName: "author",
                      arguments: null,
                    },
                    {
                      kind: "Scalar",
                      isFallible: false,
                      fieldName: "content",
                      arguments: null,
                    },
                    {
                      kind: "Linked",
                      isFallible: true,
                      fieldName: "image",
                      arguments: null,
                      concreteType: "Image",
                      selections: [
                        {
                          kind: "Scalar",
                          isFallible: false,
                          fieldName: "id",
                          arguments: null,
                        },
                      ],
                    },
                    {
                      kind: "Scalar",
                      isFallible: false,
                      fieldName: "title",
                      arguments: null,
                    },
                  ],
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
