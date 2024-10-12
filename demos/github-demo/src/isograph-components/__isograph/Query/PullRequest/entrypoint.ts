import type {IsographEntrypoint, NormalizationAst, RefetchQueryNormalizationArtifactWrapper} from '@isograph/react';
import {Query__PullRequest__param} from './param_type';
import {Query__PullRequest__output_type} from './output_type';
import readerResolver from './resolver_reader';
const nestedRefetchQueries: RefetchQueryNormalizationArtifactWrapper[] = [];

const queryText = 'query PullRequest ($repositoryOwner: String!, $repositoryName: String!, $pullRequestNumber: Int!) {\
  repository____owner___v_repositoryOwner____name___v_repositoryName: repository(owner: $repositoryOwner, name: $repositoryName) {\
    id,\
    pullRequest____number___v_pullRequestNumber: pullRequest(number: $pullRequestNumber) {\
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
        { kind: "Variable", name: "repositoryOwner" },
      ],

      [
        "name",
        { kind: "Variable", name: "repositoryName" },
      ],
    ],
    concreteType: "Repository",
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
            { kind: "Variable", name: "pullRequestNumber" },
          ],
        ],
        concreteType: "PullRequest",
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
            concreteType: "IssueCommentConnection",
            selections: [
              {
                kind: "Linked",
                fieldName: "edges",
                arguments: null,
                concreteType: "IssueCommentEdge",
                selections: [
                  {
                    kind: "Linked",
                    fieldName: "node",
                    arguments: null,
                    concreteType: "IssueComment",
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
                        concreteType: "Actor",
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
    concreteType: "User",
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
  concreteType: "Query",
  readerWithRefetchQueries: {
    kind: "ReaderWithRefetchQueries",
    nestedRefetchQueries,
    readerArtifact: readerResolver,
  },
};

export default artifact;
