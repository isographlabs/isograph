import type {NormalizationAst} from '@isograph/react';
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
  ],
};
export default normalizationAst;
