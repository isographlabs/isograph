import type {NormalizationAst} from '@isograph/react';
const normalizationAst: NormalizationAst = {
  kind: "NormalizationAst",
  selections: [
    {
      kind: "Linked",
      isFallible: true,
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
              kind: "Linked",
              isFallible: false,
              fieldName: "repositories",
              arguments: [
                [
                  "first",
                  { kind: "Variable", name: "first" },
                ],

                [
                  "after",
                  { kind: "Variable", name: "after" },
                ],
              ],
              concreteType: "RepositoryConnection",
              selections: [
                {
                  kind: "Linked",
                  isFallible: true,
                  fieldName: "edges",
                  arguments: null,
                  concreteType: "RepositoryEdge",
                  selections: [
                    {
                      kind: "Linked",
                      isFallible: true,
                      fieldName: "node",
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
                          isFallible: true,
                          fieldName: "description",
                          arguments: null,
                        },
                        {
                          kind: "Scalar",
                          isFallible: false,
                          fieldName: "forkCount",
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
                        {
                          kind: "Linked",
                          isFallible: false,
                          fieldName: "pullRequests",
                          arguments: null,
                          concreteType: "PullRequestConnection",
                          selections: [
                            {
                              kind: "Scalar",
                              isFallible: false,
                              fieldName: "totalCount",
                              arguments: null,
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
                          kind: "Linked",
                          isFallible: false,
                          fieldName: "watchers",
                          arguments: null,
                          concreteType: "UserConnection",
                          selections: [
                            {
                              kind: "Scalar",
                              isFallible: false,
                              fieldName: "totalCount",
                              arguments: null,
                            },
                          ],
                        },
                      ],
                    },
                  ],
                },
                {
                  kind: "Linked",
                  isFallible: false,
                  fieldName: "pageInfo",
                  arguments: null,
                  concreteType: "PageInfo",
                  selections: [
                    {
                      kind: "Scalar",
                      isFallible: true,
                      fieldName: "endCursor",
                      arguments: null,
                    },
                    {
                      kind: "Scalar",
                      isFallible: false,
                      fieldName: "hasNextPage",
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
export default normalizationAst;
