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
          isFallible: false,
          fieldName: "id",
          arguments: null,
        },
        {
          kind: "Linked",
          isFallible: true,
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
              isFallible: false,
              fieldName: "id",
              arguments: null,
            },
            {
              kind: "Scalar",
              isFallible: false,
              fieldName: "bodyHTML",
              arguments: null,
            },
            {
              kind: "Linked",
              isFallible: false,
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
                  isFallible: true,
                  fieldName: "edges",
                  arguments: null,
                  concreteType: "IssueCommentEdge",
                  selections: [
                    {
                      kind: "Linked",
                      isFallible: true,
                      fieldName: "node",
                      arguments: null,
                      concreteType: "IssueComment",
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
                          ],
                        },
                        {
                          kind: "Scalar",
                          isFallible: false,
                          fieldName: "bodyText",
                          arguments: null,
                        },
                        {
                          kind: "Scalar",
                          isFallible: false,
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
              isFallible: false,
              fieldName: "title",
              arguments: null,
            },
          ],
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
