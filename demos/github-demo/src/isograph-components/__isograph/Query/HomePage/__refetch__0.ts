import type {IsographEntrypoint, ReaderAst, FragmentReference, NormalizationAst, RefetchQueryNormalizationArtifact} from '@isograph/react';
const queryText = 'query User__refetch ($first: Int!, $id: ID!) {\
  node____id___v_id: node(id: $id) {\
    ... on User {\
      login,\
      avatarUrl,\
      name,\
      id,\
      repositories____last___l_10: repositories(last: 10) {\
        edges {\
          node {\
            id,\
            description,\
            forkCount,\
            name,\
            nameWithOwner,\
            owner {\
              id,\
              login,\
            },\
            pullRequests____first___v_first: pullRequests(first: $first) {\
              totalCount,\
            },\
            stargazerCount,\
            watchers____first___v_first: watchers(first: $first) {\
              totalCount,\
            },\
          },\
        },\
      },\
      __typename,\
    },\
  },\
}';

const normalizationAst: NormalizationAst = [
  {
    kind: "Linked",
    fieldName: "node",
    arguments: [
      [
        "id",
        { kind: "Variable", name: "id" },
      ],
    ],
    selections: [
      {
        kind: "InlineFragment",
        type: "User",
        selections: [
          {
            kind: "Scalar",
            fieldName: "login",
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
                "last",
                { kind: "Literal", value: 10 },
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
                        selections: [
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
                        arguments: [
                          [
                            "first",
                            { kind: "Variable", name: "first" },
                          ],
                        ],
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
                        arguments: [
                          [
                            "first",
                            { kind: "Variable", name: "first" },
                          ],
                        ],
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
            ],
          },
          {
            kind: "Scalar",
            fieldName: "__typename",
            arguments: null,
          },
        ],
      },
    ],
  },
];
const artifact: any = {
  kind: "RefetchQuery",
  queryText,
  normalizationAst,
};

export default artifact;
