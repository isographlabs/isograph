import type {IsographEntrypoint, NormalizationAst, RefetchQueryNormalizationArtifactWrapper} from '@isograph/react';
import {Query__Newsfeed__param} from './param_type';
import {Query__Newsfeed__output_type} from './output_type';
import readerResolver from './resolver_reader';
const nestedRefetchQueries: RefetchQueryNormalizationArtifactWrapper[] = [];

const queryText = 'query Newsfeed  {\
  viewer {\
    id,\
    newsfeed____skip___l_0____limit___l_6: newsfeed(skip: 0, limit: 6) {\
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
}';

const normalizationAst: NormalizationAst = {
  kind: "NormalizationAst",
  selections: [
    {
      kind: "Linked",
      fieldName: "viewer",
      arguments: null,
      concreteType: "Viewer",
      selections: [
        {
          kind: "Scalar",
          fieldName: "id",
          arguments: null,
          isUpdatable: false,
        },
        {
          kind: "Linked",
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
              fieldName: "__typename",
              arguments: null,
              isUpdatable: false,
            },
            {
              kind: "InlineFragment",
              type: "AdItem",
              selections: [
                {
                  kind: "Scalar",
                  fieldName: "id",
                  arguments: null,
                  isUpdatable: false,
                },
                {
                  kind: "Scalar",
                  fieldName: "__typename",
                  arguments: null,
                  isUpdatable: false,
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
                  isUpdatable: false,
                },
                {
                  kind: "Scalar",
                  fieldName: "__typename",
                  arguments: null,
                  isUpdatable: false,
                },
                {
                  kind: "Scalar",
                  fieldName: "author",
                  arguments: null,
                  isUpdatable: false,
                },
                {
                  kind: "Scalar",
                  fieldName: "content",
                  arguments: null,
                  isUpdatable: false,
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
                      isUpdatable: false,
                    },
                  ],
                },
                {
                  kind: "Scalar",
                  fieldName: "title",
                  arguments: null,
                  isUpdatable: false,
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
  Query__Newsfeed__param,
  Query__Newsfeed__output_type,
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
