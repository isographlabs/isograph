import type {IsographEntrypoint, NormalizationAst, RefetchQueryNormalizationArtifactWrapper} from '@isograph/react';
import {Query__RepositoryPage__param} from './param_type';
import {Query__RepositoryPage__output_type} from './output_type';
import readerResolver from './resolver_reader';
import queryText from './query_text';
const nestedRefetchQueries: RefetchQueryNormalizationArtifactWrapper[] = [];

const normalizationAst: NormalizationAst = {
  kind: "NormalizationAst",
  selections: [
    {
      kind: "Linked",
      fieldName: "repository",
      arguments: [
        [
          "name",
          { kind: "Variable", name: "repositoryName" },
        ],

        [
          "owner",
          { kind: "Variable", name: "repositoryOwner" },
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
          kind: "Scalar",
          fieldName: "nameWithOwner",
          arguments: null,
        },
        {
          kind: "Linked",
          fieldName: "parent",
          arguments: null,
          concreteType: "Repository",
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
            {
              kind: "Scalar",
              fieldName: "nameWithOwner",
              arguments: null,
            },
            {
              kind: "Linked",
              fieldName: "owner",
              arguments: null,
              concreteType: null,
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
                  kind: "Scalar",
                  fieldName: "login",
                  arguments: null,
                },
              ],
            },
          ],
        },
        {
          kind: "Linked",
          fieldName: "pullRequests",
          arguments: [
            [
              "last",
              { kind: "Variable", name: "first" },
            ],
          ],
          concreteType: "PullRequestConnection",
          selections: [
            {
              kind: "Linked",
              fieldName: "edges",
              arguments: null,
              concreteType: "PullRequestEdge",
              selections: [
                {
                  kind: "Linked",
                  fieldName: "node",
                  arguments: null,
                  concreteType: "PullRequest",
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
                      concreteType: null,
                      selections: [
                        {
                          kind: "Scalar",
                          fieldName: "__typename",
                          arguments: null,
                        },
                        {
                          kind: "Scalar",
                          fieldName: "login",
                          arguments: null,
                        },
                        {
                          kind: "InlineFragment",
                          type: "User",
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
                              fieldName: "twitterUsername",
                              arguments: null,
                            },
                          ],
                        },
                      ],
                    },
                    {
                      kind: "Scalar",
                      fieldName: "closed",
                      arguments: null,
                    },
                    {
                      kind: "Scalar",
                      fieldName: "createdAt",
                      arguments: null,
                    },
                    {
                      kind: "Scalar",
                      fieldName: "number",
                      arguments: null,
                    },
                    {
                      kind: "Linked",
                      fieldName: "repository",
                      arguments: null,
                      concreteType: "Repository",
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
                        {
                          kind: "Linked",
                          fieldName: "owner",
                          arguments: null,
                          concreteType: null,
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
                              kind: "Scalar",
                              fieldName: "login",
                              arguments: null,
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
                    {
                      kind: "Scalar",
                      fieldName: "totalCommentsCount",
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
          fieldName: "stargazerCount",
          arguments: null,
        },
        {
          kind: "Scalar",
          fieldName: "viewerHasStarred",
          arguments: null,
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
  ],
};
const artifact: IsographEntrypoint<
  Query__RepositoryPage__param,
  Query__RepositoryPage__output_type,
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
