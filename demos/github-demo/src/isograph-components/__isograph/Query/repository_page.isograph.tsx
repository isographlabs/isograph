import type {IsographFetchableResolver, ReaderAst, FragmentReference, NormalizationAst} from '@isograph/react';
const resolver = x => x;
import Query__header, { ReadOutType as Query__header__outputType } from './header.isograph';
import Query__repository_detail, { ReadOutType as Query__repository_detail__outputType } from './repository_detail.isograph';

const nestedRefetchQueries = [];

const queryText = 'query repository_page ($repositoryName: String!, $repositoryOwner: String!, $first: Int!) {\
  repository____name___repositoryName____owner___repositoryOwner: repository(name: $repositoryName, owner: $repositoryOwner) {\
    id,\
    nameWithOwner,\
    parent {\
      id,\
      name,\
      nameWithOwner,\
      owner {\
        id,\
        login,\
      },\
    },\
    pullRequests____last___first: pullRequests(last: $first) {\
      edges {\
        node {\
          id,\
          author {\
            login,\
          },\
          closed,\
          createdAt,\
          number,\
          repository {\
            id,\
            name,\
            owner {\
              id,\
              login,\
            },\
          },\
          title,\
          totalCommentsCount,\
        },\
      },\
    },\
  },\
  viewer {\
    id,\
    avatarUrl,\
    name,\
  },\
}';

// TODO support changing this,
export type ReadFromStoreType = ResolverParameterType;

const normalizationAst: NormalizationAst = [
  {
    kind: "Linked",
    fieldName: "repository",
    arguments: [
      {
        argumentName: "name",
        variableName: "repositoryName",
      },

      {
        argumentName: "owner",
        variableName: "repositoryOwner",
      },
    ],
    selections: [
      {
        kind: "Scalar",
        fieldName: "id",
        arguments: null,
      },
      {
        kind: "Scalar",
        fieldName: "nameWithOwner",
        arguments: null,
      },
      {
        kind: "Linked",
        fieldName: "parent",
        arguments: null,
        selections: [
          {
            kind: "Scalar",
            fieldName: "id",
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
        ],
      },
      {
        kind: "Linked",
        fieldName: "pullRequests",
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
                    kind: "Linked",
                    fieldName: "author",
                    arguments: null,
                    selections: [
                      {
                        kind: "Scalar",
                        fieldName: "login",
                        arguments: null,
                      },
                    ],
                  },
                  {
                    kind: "Scalar",
                    fieldName: "closed",
                    arguments: null,
                  },
                  {
                    kind: "Scalar",
                    fieldName: "createdAt",
                    arguments: null,
                  },
                  {
                    kind: "Scalar",
                    fieldName: "number",
                    arguments: null,
                  },
                  {
                    kind: "Linked",
                    fieldName: "repository",
                    arguments: null,
                    selections: [
                      {
                        kind: "Scalar",
                        fieldName: "id",
                        arguments: null,
                      },
                      {
                        kind: "Scalar",
                        fieldName: "name",
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
                    ],
                  },
                  {
                    kind: "Scalar",
                    fieldName: "title",
                    arguments: null,
                  },
                  {
                    kind: "Scalar",
                    fieldName: "totalCommentsCount",
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
    fieldName: "viewer",
    arguments: null,
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
];
const readerAst: ReaderAst<ReadFromStoreType> = [
  {
    kind: "Resolver",
    alias: "header",
    arguments: null,
    resolver: Query__header,
    variant: "Component",
    usedRefetchQueries: [0, ],
  },
  {
    kind: "Resolver",
    alias: "repository_detail",
    arguments: null,
    resolver: Query__repository_detail,
    variant: "Component",
    usedRefetchQueries: [0, ],
  },
];

export type ResolverParameterType = {
  header: Query__header__outputType,
  repository_detail: Query__repository_detail__outputType,
};

// The type, when returned from the resolver
export type ResolverReturnType = ResolverParameterType;

// the type, when read out (either via useLazyReference or via graph)
export type ReadOutType = ResolverReturnType;

const artifact: IsographFetchableResolver<ReadFromStoreType, ResolverParameterType, ReadOutType> = {
  kind: 'FetchableResolver',
  queryText,
  normalizationAst,
  readerAst,
  resolver: resolver as any,
  convert: ((resolver, data) => resolver(data)),
  nestedRefetchQueries,
};

export default artifact;
