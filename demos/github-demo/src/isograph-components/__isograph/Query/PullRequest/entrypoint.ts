import type {IsographEntrypoint, NormalizationAst, RefetchQueryNormalizationArtifactWrapper} from '@isograph/react';
import {Query__PullRequest__param} from './param_type';
import {Query__PullRequest__output_type} from './output_type';
import readerResolver from './resolver_reader';
const nestedRefetchQueries: RefetchQueryNormalizationArtifactWrapper[] = [];

const queryText = 'query PullRequest ($repositoryOwner: String!, $repositoryName: String!, $pullRequestNumber: Int!, $last: Int!) {\
  repository____owner___l_null____name___l_null: repository(owner: null, name: null) {\
    id,\
    pullRequest____number___l_null: pullRequest(number: null) {\
      id,\
      bodyHTML,\
      comments____last___l_null: comments(last: null) {\
        edges {\
          node {\
            id,\
            author {\
              login,\
            },\
            bodyText,\
            createdAt,\
          },\
        },\
      },\
      title,\
    },\
  },\
  viewer {\
    id,\
    avatarUrl,\
    name,\
  },\
}';

const normalizationAst: NormalizationAst = [
  {
    kind: "Linked",
    fieldName: "repository",
    arguments: [
      [
        "owner",
        { kind: "Literal", value: null },
      ],

      [
        "name",
        { kind: "Literal", value: null },
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
        fieldName: "pullRequest",
        arguments: [
          [
            "number",
            { kind: "Literal", value: null },
          ],
        ],
        selections: [
          {
            kind: "Scalar",
            fieldName: "id",
            arguments: null,
          },
          {
            kind: "Scalar",
            fieldName: "bodyHTML",
            arguments: null,
          },
          {
            kind: "Linked",
            fieldName: "comments",
            arguments: [
              [
                "last",
                { kind: "Literal", value: null },
              ],
            ],
            selections: [
              {
                kind: "Linked",
                fieldName: "edges",
                arguments: null,
                selections: [
                  {
                    kind: "Linked",
                    fieldName: "node",
                    arguments: null,
                    selections: [
                      {
                        kind: "Scalar",
                        fieldName: "id",
                        arguments: null,
                      },
                      {
                        kind: "Linked",
                        fieldName: "author",
                        arguments: null,
                        selections: [
                          {
                            kind: "Scalar",
                            fieldName: "login",
                            arguments: null,
                          },
                        ],
                      },
                      {
                        kind: "Scalar",
                        fieldName: "bodyText",
                        arguments: null,
                      },
                      {
                        kind: "Scalar",
                        fieldName: "createdAt",
                        arguments: null,
                      },
                    ],
                  },
                ],
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
        kind: "Scalar",
        fieldName: "avatarUrl",
        arguments: null,
      },
      {
        kind: "Scalar",
        fieldName: "name",
        arguments: null,
      },
    ],
  },
];
const artifact: IsographEntrypoint<
  Query__PullRequest__param,
  Query__PullRequest__output_type
> = {
  kind: "Entrypoint",
  queryText,
  normalizationAst,
  nestedRefetchQueries,
  readerArtifact: readerResolver,
};

export default artifact;
