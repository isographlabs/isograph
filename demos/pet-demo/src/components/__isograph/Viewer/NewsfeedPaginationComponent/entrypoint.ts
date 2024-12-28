import type {IsographEntrypoint, NormalizationAst, RefetchQueryNormalizationArtifactWrapper} from '@isograph/react';
import {Viewer__NewsfeedPaginationComponent__param} from './param_type';
import {Viewer__NewsfeedPaginationComponent__output_type} from './output_type';
import readerResolver from './resolver_reader';
const nestedRefetchQueries: RefetchQueryNormalizationArtifactWrapper[] = [];

const queryText = 'query NewsfeedPaginationComponent ($skip: Int!, $limit: Int!, $id: ID!) {\
  node____id___v_id: node(id: $id) {\
    ... on Viewer {\
      __typename,\
      id,\
      newsfeed____skip___v_skip____limit___v_limit: newsfeed(skip: $skip, limit: $limit) {\
        __typename,\
        ... on AdItem {\
          id,\
          __typename,\
        },\
        ... on BlogItem {\
          id,\
          __typename,\
          author,\
          content,\
          image {\
            id,\
          },\
          title,\
        },\
      },\
    },\
  },\
}';

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
          type: "Viewer",
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
                  fieldName: "__typename",
                  arguments: null,
                },
                {
                  kind: "InlineFragment",
                  type: "AdItem",
                  selections: [
                    {
                      kind: "Scalar",
                      fieldName: "id",
                      arguments: null,
                    },
                    {
                      kind: "Scalar",
                      fieldName: "__typename",
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
                      fieldName: "id",
                      arguments: null,
                    },
                    {
                      kind: "Scalar",
                      fieldName: "__typename",
                      arguments: null,
                    },
                    {
                      kind: "Scalar",
                      fieldName: "author",
                      arguments: null,
                    },
                    {
                      kind: "Scalar",
                      fieldName: "content",
                      arguments: null,
                    },
                    {
                      kind: "Linked",
                      fieldName: "image",
                      arguments: null,
                      concreteType: "Image",
                      selections: [
                        {
                          kind: "Scalar",
                          fieldName: "id",
                          arguments: null,
                        },
                      ],
                    },
                    {
                      kind: "Scalar",
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
const artifact: IsographEntrypoint<
  Viewer__NewsfeedPaginationComponent__param,
  Viewer__NewsfeedPaginationComponent__output_type,
  NormalizationAst
> = {
  kind: "Entrypoint",
  networkRequestInfo: {
    kind: "NetworkRequestInfo",
    queryText,
    normalizationAst,
  },
  concreteType: "Query",
  readerWithRefetchQueries: {
    kind: "ReaderWithRefetchQueries",
    nestedRefetchQueries,
    readerArtifact: readerResolver,
  },
};

export default artifact;
