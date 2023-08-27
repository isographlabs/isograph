import type {IsographFetchableResolver, ReaderAst, FragmentReference} from '@isograph/react';
import { getRefRendererForName } from '@isograph/react';
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

const normalizationAst = [
  {
    kind: "Linked",
    field_name: "repository",
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
        field_name: "id",
        alias: null,
        arguments: null,
      },
      {
        kind: "Scalar",
        field_name: "nameWithOwner",
        alias: null,
        arguments: null,
      },
      {
        kind: "Linked",
        field_name: "parent",
        alias: null,
        arguments: null,
        selections: [
          {
            kind: "Scalar",
            field_name: "id",
            alias: null,
            arguments: null,
          },
          {
            kind: "Scalar",
            field_name: "name",
            alias: null,
            arguments: null,
          },
          {
            kind: "Scalar",
            field_name: "nameWithOwner",
            alias: null,
            arguments: null,
          },
          {
            kind: "Linked",
            field_name: "owner",
            alias: null,
            arguments: null,
            selections: [
              {
                kind: "Scalar",
                field_name: "id",
                alias: null,
                arguments: null,
              },
              {
                kind: "Scalar",
                field_name: "login",
                alias: null,
                arguments: null,
              },
            ],
          },
        ],
      },
      {
        kind: "Linked",
        field_name: "pullRequests",
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
            field_name: "edges",
            alias: null,
            arguments: null,
            selections: [
              {
                kind: "Linked",
                field_name: "node",
                alias: null,
                arguments: null,
                selections: [
                  {
                    kind: "Scalar",
                    field_name: "id",
                    alias: null,
                    arguments: null,
                  },
                  {
                    kind: "Linked",
                    field_name: "author",
                    alias: null,
                    arguments: null,
                    selections: [
                      {
                        kind: "Scalar",
                        field_name: "login",
                        alias: null,
                        arguments: null,
                      },
                    ],
                  },
                  {
                    kind: "Scalar",
                    field_name: "closed",
                    alias: null,
                    arguments: null,
                  },
                  {
                    kind: "Scalar",
                    field_name: "createdAt",
                    alias: null,
                    arguments: null,
                  },
                  {
                    kind: "Scalar",
                    field_name: "number",
                    alias: null,
                    arguments: null,
                  },
                  {
                    kind: "Linked",
                    field_name: "repository",
                    alias: null,
                    arguments: null,
                    selections: [
                      {
                        kind: "Scalar",
                        field_name: "id",
                        alias: null,
                        arguments: null,
                      },
                      {
                        kind: "Scalar",
                        field_name: "name",
                        alias: null,
                        arguments: null,
                      },
                      {
                        kind: "Linked",
                        field_name: "owner",
                        alias: null,
                        arguments: null,
                        selections: [
                          {
                            kind: "Scalar",
                            field_name: "id",
                            alias: null,
                            arguments: null,
                          },
                          {
                            kind: "Scalar",
                            field_name: "login",
                            alias: null,
                            arguments: null,
                          },
                        ],
                      },
                    ],
                  },
                  {
                    kind: "Scalar",
                    field_name: "title",
                    alias: null,
                    arguments: null,
                  },
                  {
                    kind: "Scalar",
                    field_name: "totalCommentsCount",
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
    field_name: "viewer",
    alias: null,
    arguments: null,
    selections: [
      {
        kind: "Scalar",
        field_name: "id",
        alias: null,
        arguments: null,
      },
      {
        kind: "Scalar",
        field_name: "avatarUrl",
        alias: null,
        arguments: null,
      },
      {
        kind: "Scalar",
        field_name: "name",
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
