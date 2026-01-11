import type {NormalizationAst} from '@isograph/react';
const normalizationAst: NormalizationAst = {
  kind: "NormalizationAst",
  selections: [
    {
      kind: "Linked",
      isFallible: true,
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
          isFallible: false,
          fieldName: "id",
          arguments: null,
        },
        {
          kind: "Scalar",
          isFallible: false,
          fieldName: "nameWithOwner",
          arguments: null,
        },
        {
          kind: "Linked",
          isFallible: true,
          fieldName: "parent",
          arguments: null,
          concreteType: "Repository",
          selections: [
            {
              kind: "Scalar",
              isFallible: false,
              fieldName: "id",
              arguments: null,
            },
            {
              kind: "Scalar",
              isFallible: false,
              fieldName: "name",
              arguments: null,
            },
            {
              kind: "Scalar",
              isFallible: false,
              fieldName: "nameWithOwner",
              arguments: null,
            },
            {
              kind: "Linked",
              isFallible: false,
              fieldName: "owner",
              arguments: null,
              concreteType: null,
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
                  fieldName: "login",
                  arguments: null,
                },
              ],
            },
          ],
        },
        {
          kind: "Linked",
          isFallible: false,
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
              isFallible: true,
              fieldName: "edges",
              arguments: null,
              concreteType: "PullRequestEdge",
              selections: [
                {
                  kind: "Linked",
                  isFallible: true,
                  fieldName: "node",
                  arguments: null,
                  concreteType: "PullRequest",
                  selections: [
                    {
                      kind: "Scalar",
                      isFallible: false,
                      fieldName: "id",
                      arguments: null,
                    },
                    {
                      kind: "Linked",
                      isFallible: true,
                      fieldName: "author",
                      arguments: null,
                      concreteType: null,
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
                          fieldName: "login",
                          arguments: null,
                        },
                        {
                          kind: "InlineFragment",
                          type: "User",
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
                              isFallible: true,
                              fieldName: "twitterUsername",
                              arguments: null,
                            },
                          ],
                        },
                      ],
                    },
                    {
                      kind: "Scalar",
                      isFallible: false,
                      fieldName: "closed",
                      arguments: null,
                    },
                    {
                      kind: "Scalar",
                      isFallible: false,
                      fieldName: "createdAt",
                      arguments: null,
                    },
                    {
                      kind: "Scalar",
                      isFallible: false,
                      fieldName: "number",
                      arguments: null,
                    },
                    {
                      kind: "Linked",
                      isFallible: false,
                      fieldName: "repository",
                      arguments: null,
                      concreteType: "Repository",
                      selections: [
                        {
                          kind: "Scalar",
                          isFallible: false,
                          fieldName: "id",
                          arguments: null,
                        },
                        {
                          kind: "Scalar",
                          isFallible: false,
                          fieldName: "name",
                          arguments: null,
                        },
                        {
                          kind: "Linked",
                          isFallible: false,
                          fieldName: "owner",
                          arguments: null,
                          concreteType: null,
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
                              fieldName: "login",
                              arguments: null,
                            },
                          ],
                        },
                      ],
                    },
                    {
                      kind: "Scalar",
                      isFallible: false,
                      fieldName: "title",
                      arguments: null,
                    },
                    {
                      kind: "Scalar",
                      isFallible: true,
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
          isFallible: false,
          fieldName: "stargazerCount",
          arguments: null,
        },
        {
          kind: "Scalar",
          isFallible: false,
          fieldName: "viewerHasStarred",
          arguments: null,
        },
      ],
    },
    {
      kind: "Linked",
      isFallible: false,
      fieldName: "viewer",
      arguments: null,
      concreteType: "User",
      selections: [
        {
          kind: "Scalar",
          isFallible: false,
          fieldName: "id",
          arguments: null,
        },
        {
          kind: "Scalar",
          isFallible: false,
          fieldName: "avatarUrl",
          arguments: null,
        },
        {
          kind: "Scalar",
          isFallible: true,
          fieldName: "name",
          arguments: null,
        },
      ],
    },
  ],
};
export default normalizationAst;
