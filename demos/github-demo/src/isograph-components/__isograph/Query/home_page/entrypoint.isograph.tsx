import type {IsographFetchableResolver, FragmentReference, NormalizationAst, RefetchQueryArtifactWrapper} from '@isograph/react';
import type {ReadFromStoreType, ResolverParameterType, ReadOutType} from './reader.isograph';
import readerResolver from './reader.isograph';
import refetchQuery0 from './__refetch__0.isograph';
import refetchQuery1 from './__refetch__1.isograph';
const nestedRefetchQueries: RefetchQueryArtifactWrapper[] = [{ artifact: refetchQuery0, allowedVariables: ["first", ] }, { artifact: refetchQuery1, allowedVariables: ["first", ] }, ];

const queryText = 'query home_page ($first: Int!) {\
  viewer {\
    avatarUrl,\
    name,\
    login,\
    id,\
    status {\
      id,\
      emoji,\
      user {\
        id,\
        repositories____last___first: repositories(last: $first) {\
          __typename,\
        },\
      },\
    },\
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
  },\
}';

const normalizationAst: NormalizationAst = [
  {
    kind: "Linked",
    fieldName: "viewer",
    arguments: null,
    selections: [
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
        fieldName: "login",
        arguments: null,
      },
      {
        kind: "Scalar",
        fieldName: "id",
        arguments: null,
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
          {
            kind: "Linked",
            fieldName: "user",
            arguments: null,
            selections: [
              {
                kind: "Scalar",
                fieldName: "id",
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
                    kind: "Scalar",
                    fieldName: "__typename",
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
    ],
  },
];
const artifact: IsographFetchableResolver<ReadFromStoreType, ResolverParameterType, ReadOutType> = {
  kind: 'FetchableResolver',
  queryText,
  normalizationAst,
  nestedRefetchQueries,
  readerAst: readerResolver.readerAst,
  resolver: readerResolver.resolver,
};

export default artifact;
