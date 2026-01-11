import type {NormalizationAst} from '@isograph/react';
const normalizationAst: NormalizationAst = {
  kind: "NormalizationAst",
  selections: [
    {
      kind: "Linked",
      isFallible: false,
      fieldName: "viewer",
      arguments: null,
      concreteType: "Viewer",
      selections: [
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
              { kind: "Literal", value: 0 },
            ],

            [
              "limit",
              { kind: "Literal", value: 6 },
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
};
export default normalizationAst;
