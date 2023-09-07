import type {IsographFetchableResolver, ReaderAst, FragmentReference, NormalizationAst} from '@isograph/react';
const queryText = 'query User_refetch ($first: Int!, $id: ID!) { node____id___id: node(id: $id) { ... on User { \
  avatarUrl,\
  id,\
  login,\
  name,\
  repositories____last___first: repositories(last: $first) {\
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
        pullRequests____first___first: pullRequests(first: $first) {\
          totalCount,\
        },\
        stargazerCount,\
        watchers____first___first: watchers(first: $first) {\
          totalCount,\
        },\
      },\
    },\
  },\
  status {\
    id,\
    emoji,\
  },\
}}}';

const normalizationAst: NormalizationAst = [{ kind: "Linked", fieldName: "node", alias: null, arguments: [{ argumentName: "id", variableName: "id" }], selections: [
  {
    kind: "Scalar",
    fieldName: "avatarUrl",
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
  {
    kind: "Scalar",
    fieldName: "name",
    arguments: null,
  },
  {
    kind: "Linked",
    fieldName: "repositories",
    arguments: [
      {
        argumentName: "last",
        variableName: "first",
      },
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
                  {
                    argumentName: "first",
                    variableName: "first",
                  },
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
                  {
                    argumentName: "first",
                    variableName: "first",
                  },
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
    kind: "Linked",
    fieldName: "status",
    arguments: null,
    selections: [
      {
        kind: "Scalar",
        fieldName: "id",
        arguments: null,
      },
      {
        kind: "Scalar",
        fieldName: "emoji",
        arguments: null,
      },
    ],
  },
] }];
const artifact: any = {
  kind: "RefetchQuery",
  queryText,
  normalizationAst,
};

export default artifact;
