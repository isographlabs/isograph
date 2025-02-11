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
      comments____last___l_10: comments(last: 10) {\
        edges {\
          node {\
            id,\
            author {\
              __typename,\
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

const normalizationAst: NormalizationAst = {
  kind: "NormalizationAst",
  selections: [
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
          isUpdatable: false,
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
              isUpdatable: false,
            },
            {
              kind: "Scalar",
              fieldName: "bodyHTML",
              arguments: null,
              isUpdatable: false,
            },
            {
              kind: "Linked",
              fieldName: "comments",
              arguments: [
                [
                  "last",
                  { kind: "Literal", value: 10 },
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
                          isUpdatable: false,
                        },
                        {
                          kind: "Linked",
                          fieldName: "author",
                          arguments: null,
                          concreteType: null,
                          selections: [
                            {
                              kind: "Scalar",
                              fieldName: "__typename",
                              arguments: null,
                              isUpdatable: false,
                            },
                            {
                              kind: "Scalar",
                              fieldName: "login",
                              arguments: null,
                              isUpdatable: false,
                            },
                          ],
                        },
                        {
                          kind: "Scalar",
                          fieldName: "bodyText",
                          arguments: null,
                          isUpdatable: false,
                        },
                        {
                          kind: "Scalar",
                          fieldName: "createdAt",
                          arguments: null,
                          isUpdatable: false,
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
              isUpdatable: false,
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
          isUpdatable: false,
        },
        {
          kind: "Scalar",
          fieldName: "avatarUrl",
          arguments: null,
          isUpdatable: false,
        },
        {
          kind: "Scalar",
          fieldName: "name",
          arguments: null,
          isUpdatable: false,
        },
      ],
    },
  ],
};
const artifact: IsographEntrypoint<
  Query__PullRequest__param,
  Query__PullRequest__output_type,
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
