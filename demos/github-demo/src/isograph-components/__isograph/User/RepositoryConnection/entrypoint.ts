import type {IsographEntrypoint, NormalizationAst, RefetchQueryNormalizationArtifactWrapper} from '@isograph/react';
import {User__RepositoryConnection__param} from './param_type';
import {User__RepositoryConnection__output_type} from './output_type';
import readerResolver from './resolver_reader';
const nestedRefetchQueries: RefetchQueryNormalizationArtifactWrapper[] = [];

const queryText = 'query RepositoryConnection ($first: Int, $after: String, $id: ID!) {\
  node____id___v_id: node(id: $id) {\
    ... on User {\
      __typename,\
      id,\
      repositories____first___v_first____after___v_after: repositories(first: $first, after: $after) {\
        edges {\
          node {\
            id,\
            description,\
            forkCount,\
            name,\
            nameWithOwner,\
            owner {\
              __typename,\
              id,\
              login,\
            },\
            pullRequests {\
              totalCount,\
            },\
            stargazerCount,\
            watchers {\
              totalCount,\
            },\
          },\
        },\
        pageInfo {\
          endCursor,\
          hasNextPage,\
        },\
      },\
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
];
const artifact: IsographEntrypoint<
  User__RepositoryConnection__param,
  User__RepositoryConnection__output_type
> = {
  kind: "Entrypoint",
  queryText,
  normalizationAst,
  readerWithRefetchQueries: {
    kind: "ReaderWithRefetchQueries",
    nestedRefetchQueries,
    readerArtifact: readerResolver,
  },
};

export default artifact;
