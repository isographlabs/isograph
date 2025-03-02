import type {NormalizationAst} from '@isograph/react';
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
          type: "User",
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
                  fieldName: "edges",
                  arguments: null,
                  concreteType: "RepositoryEdge",
                  selections: [
                    {
                      kind: "Linked",
                      fieldName: "node",
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
                          fieldName: "description",
                          arguments: null,
                        },
                        {
                          kind: "Scalar",
                          fieldName: "forkCount",
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
                        {
                          kind: "Linked",
                          fieldName: "pullRequests",
                          arguments: null,
                          concreteType: "PullRequestConnection",
                          selections: [
                            {
                              kind: "Scalar",
                              fieldName: "totalCount",
                              arguments: null,
                            },
                          ],
                        },
                        {
                          kind: "Scalar",
                          fieldName: "stargazerCount",
                          arguments: null,
                        },
                        {
                          kind: "Linked",
                          fieldName: "watchers",
                          arguments: null,
                          concreteType: "UserConnection",
                          selections: [
                            {
                              kind: "Scalar",
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
                  fieldName: "pageInfo",
                  arguments: null,
                  concreteType: "PageInfo",
                  selections: [
                    {
                      kind: "Scalar",
                      fieldName: "endCursor",
                      arguments: null,
                    },
                    {
                      kind: "Scalar",
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
