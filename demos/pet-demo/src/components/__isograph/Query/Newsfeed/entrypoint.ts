import type {IsographEntrypoint, NormalizationAst, RefetchQueryNormalizationArtifactWrapper} from '@isograph/react';
import {Query__Newsfeed__param} from './param_type';
import {Query__Newsfeed__output_type} from './output_type';
import readerResolver from './resolver_reader';
const nestedRefetchQueries: RefetchQueryNormalizationArtifactWrapper[] = [];

const queryText = 'query Newsfeed  {\
  viewer {\
    id,\
    newsfeed____skip___l_0____limit___l_6: newsfeed(skip: 0, limit: 6) {\
      id,\
      adItem {\
        id,\
      },\
      blogItem {\
        id,\
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

const normalizationAst: NormalizationAst = [
  {
    kind: "Linked",
    fieldName: "viewer",
    arguments: null,
    selections: [
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
            { kind: "Literal", value: 0 },
          ],

          [
            "limit",
            { kind: "Literal", value: 6 },
          ],
        ],
        selections: [
          {
            kind: "Scalar",
            fieldName: "id",
            arguments: null,
          },
          {
            kind: "Linked",
            fieldName: "adItem",
            arguments: null,
            selections: [
              {
                kind: "Scalar",
                fieldName: "id",
                arguments: null,
              },
            ],
          },
          {
            kind: "Linked",
            fieldName: "blogItem",
            arguments: null,
            selections: [
              {
                kind: "Scalar",
                fieldName: "id",
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
];
const artifact: IsographEntrypoint<
  Query__Newsfeed__param,
  Query__Newsfeed__output_type
> = {
  kind: "Entrypoint",
  queryText,
  normalizationAst,
  readerWithRefetchQueries: {
    kind: "ReaderWithRefetchQueries",
    nestedRefetchQueries,
    readerArtifact: readerResolver,
  },
};

export default artifact;
