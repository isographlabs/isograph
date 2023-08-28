import type {IsographFetchableResolver, ReaderAst, FragmentReference, NormalizationAst} from '@isograph/react';
import {getRefRendererForName} from '@isograph/react';
const resolver = x => x;
import Query__header, { ReadOutType as Query__header__outputType } from './header.isograph';
import Query__repository_detail, { ReadOutType as Query__repository_detail__outputType } from './repository_detail.isograph';

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
    alias: "repository____name___repositoryName____owner___repositoryOwner",
    arguments: [
      {
        argument_name: "name",
        variable_name: "repositoryName",
      },

      {
        argument_name: "owner",
        variable_name: "repositoryOwner",
      },
    ],
    selections: [
      {
        kind: "Scalar",
        fieldName: "id",
        alias: null,
        arguments: null,
      },
      {
        kind: "Scalar",
        fieldName: "nameWithOwner",
        alias: null,
        arguments: null,
      },
      {
        kind: "Linked",
        fieldName: "parent",
        alias: null,
        arguments: null,
        selections: [
          {
            kind: "Scalar",
            fieldName: "id",
            alias: null,
            arguments: null,
          },
          {
            kind: "Scalar",
            fieldName: "name",
            alias: null,
            arguments: null,
          },
          {
            kind: "Scalar",
            fieldName: "nameWithOwner",
            alias: null,
            arguments: null,
          },
          {
            kind: "Linked",
            fieldName: "owner",
            alias: null,
            arguments: null,
            selections: [
              {
                kind: "Scalar",
                fieldName: "id",
                alias: null,
                arguments: null,
              },
              {
                kind: "Scalar",
                fieldName: "login",
                alias: null,
                arguments: null,
              },
            ],
          },
        ],
      },
      {
        kind: "Linked",
        fieldName: "pullRequests",
        alias: "pullRequests____last___first",
        arguments: [
          {
            argument_name: "last",
            variable_name: "first",
          },
        ],
        selections: [
          {
            kind: "Linked",
            fieldName: "edges",
            alias: null,
            arguments: null,
            selections: [
              {
                kind: "Linked",
                fieldName: "node",
                alias: null,
                arguments: null,
                selections: [
                  {
                    kind: "Scalar",
                    fieldName: "id",
                    alias: null,
                    arguments: null,
                  },
                  {
                    kind: "Linked",
                    fieldName: "author",
                    alias: null,
                    arguments: null,
                    selections: [
                      {
                        kind: "Scalar",
                        fieldName: "login",
                        alias: null,
                        arguments: null,
                      },
                    ],
                  },
                  {
                    kind: "Scalar",
                    fieldName: "closed",
                    alias: null,
                    arguments: null,
                  },
                  {
                    kind: "Scalar",
                    fieldName: "createdAt",
                    alias: null,
                    arguments: null,
                  },
                  {
                    kind: "Scalar",
                    fieldName: "number",
                    alias: null,
                    arguments: null,
                  },
                  {
                    kind: "Linked",
                    fieldName: "repository",
                    alias: null,
                    arguments: null,
                    selections: [
                      {
                        kind: "Scalar",
                        fieldName: "id",
                        alias: null,
                        arguments: null,
                      },
                      {
                        kind: "Scalar",
                        fieldName: "name",
                        alias: null,
                        arguments: null,
                      },
                      {
                        kind: "Linked",
                        fieldName: "owner",
                        alias: null,
                        arguments: null,
                        selections: [
                          {
                            kind: "Scalar",
                            fieldName: "id",
                            alias: null,
                            arguments: null,
                          },
                          {
                            kind: "Scalar",
                            fieldName: "login",
                            alias: null,
                            arguments: null,
                          },
                        ],
                      },
                    ],
                  },
                  {
                    kind: "Scalar",
                    fieldName: "title",
                    alias: null,
                    arguments: null,
                  },
                  {
                    kind: "Scalar",
                    fieldName: "totalCommentsCount",
                    alias: null,
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
    alias: null,
    arguments: null,
    selections: [
      {
        kind: "Scalar",
        fieldName: "id",
        alias: null,
        arguments: null,
      },
      {
        kind: "Scalar",
        fieldName: "avatarUrl",
        alias: null,
        arguments: null,
      },
      {
        kind: "Scalar",
        fieldName: "name",
        alias: null,
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
  },
  {
    kind: "Resolver",
    alias: "repository_detail",
    arguments: null,
    resolver: Query__repository_detail,
    variant: "Component",
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
};

export default artifact;
