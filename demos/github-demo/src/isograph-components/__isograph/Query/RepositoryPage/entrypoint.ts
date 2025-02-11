import type {IsographEntrypoint, NormalizationAst, RefetchQueryNormalizationArtifactWrapper} from '@isograph/react';
import {Query__RepositoryPage__param} from './param_type';
import {Query__RepositoryPage__output_type} from './output_type';
import readerResolver from './resolver_reader';
const nestedRefetchQueries: RefetchQueryNormalizationArtifactWrapper[] = [];

const queryText = 'query RepositoryPage ($repositoryName: String!, $repositoryOwner: String!, $first: Int!) {\
  repository____name___v_repositoryName____owner___v_repositoryOwner: repository(name: $repositoryName, owner: $repositoryOwner) {\
    id,\
    nameWithOwner,\
    parent {\
      id,\
      name,\
      nameWithOwner,\
      owner {\
        __typename,\
        id,\
        login,\
      },\
    },\
    pullRequests____last___v_first: pullRequests(last: $first) {\
      edges {\
        node {\
          id,\
          author {\
            __typename,\
            login,\
            ... on User {\
              id,\
              __typename,\
              twitterUsername,\
            },\
          },\
          closed,\
          createdAt,\
          number,\
          repository {\
            id,\
            name,\
            owner {\
              __typename,\
              id,\
              login,\
            },\
          },\
          title,\
          totalCommentsCount,\
        },\
      },\
    },\
    stargazerCount,\
    viewerHasStarred,\
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
          isUpdatable: false,
        },
        {
          kind: "Scalar",
          fieldName: "nameWithOwner",
          arguments: null,
          isUpdatable: false,
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
              isUpdatable: false,
            },
            {
              kind: "Scalar",
              fieldName: "name",
              arguments: null,
              isUpdatable: false,
            },
            {
              kind: "Scalar",
              fieldName: "nameWithOwner",
              arguments: null,
              isUpdatable: false,
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
                  isUpdatable: false,
                },
                {
                  kind: "Scalar",
                  fieldName: "id",
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
                        {
                          kind: "InlineFragment",
                          type: "User",
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
                              fieldName: "twitterUsername",
                              arguments: null,
                              isUpdatable: false,
                            },
                          ],
                        },
                      ],
                    },
                    {
                      kind: "Scalar",
                      fieldName: "closed",
                      arguments: null,
                      isUpdatable: false,
                    },
                    {
                      kind: "Scalar",
                      fieldName: "createdAt",
                      arguments: null,
                      isUpdatable: false,
                    },
                    {
                      kind: "Scalar",
                      fieldName: "number",
                      arguments: null,
                      isUpdatable: false,
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
                          isUpdatable: false,
                        },
                        {
                          kind: "Scalar",
                          fieldName: "name",
                          arguments: null,
                          isUpdatable: false,
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
                              isUpdatable: false,
                            },
                            {
                              kind: "Scalar",
                              fieldName: "id",
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
                      ],
                    },
                    {
                      kind: "Scalar",
                      fieldName: "title",
                      arguments: null,
                      isUpdatable: false,
                    },
                    {
                      kind: "Scalar",
                      fieldName: "totalCommentsCount",
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
          fieldName: "stargazerCount",
          arguments: null,
          isUpdatable: false,
        },
        {
          kind: "Scalar",
          fieldName: "viewerHasStarred",
          arguments: null,
          isUpdatable: false,
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
