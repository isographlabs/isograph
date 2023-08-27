import type {IsographFetchableResolver, ReaderAst, FragmentReference} from '@isograph/react';
import { getRefRendererForName } from '@isograph/react';
const resolver = x => x;
import Query__header, { ReadOutType as Query__header__outputType } from './header.isograph';
import Query__user_detail, { ReadOutType as Query__user_detail__outputType } from './user_detail.isograph';

const queryText = 'query user_page ($first: Int!, $userLogin: String!) {\
  user____login___userLogin: user(login: $userLogin) {\
    id,\
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
    field_name: "user",
    alias: "user____login___userLogin",
    arguments: [
      {
        argument_name: "login",
        variable_name: "userLogin",
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
        field_name: "name",
        alias: null,
        arguments: null,
      },
      {
        kind: "Linked",
        field_name: "repositories",
        alias: "repositories____last___first",
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
                    kind: "Scalar",
                    field_name: "description",
                    alias: null,
                    arguments: null,
                  },
                  {
                    kind: "Scalar",
                    field_name: "forkCount",
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
                  {
                    kind: "Linked",
                    field_name: "pullRequests",
                    alias: "pullRequests____first___first",
                    arguments: [
                      {
                        argument_name: "first",
                        variable_name: "first",
                      },
                    ],
                    selections: [
                      {
                        kind: "Scalar",
                        field_name: "totalCount",
                        alias: null,
                        arguments: null,
                      },
                    ],
                  },
                  {
                    kind: "Scalar",
                    field_name: "stargazerCount",
                    alias: null,
                    arguments: null,
                  },
                  {
                    kind: "Linked",
                    field_name: "watchers",
                    alias: "watchers____first___first",
                    arguments: [
                      {
                        argument_name: "first",
                        variable_name: "first",
                      },
                    ],
                    selections: [
                      {
                        kind: "Scalar",
                        field_name: "totalCount",
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
    alias: "user_detail",
    arguments: null,
    resolver: Query__user_detail,
    variant: "Component",
  },
];

export type ResolverParameterType = {
  header: Query__header__outputType,
  user_detail: Query__user_detail__outputType,
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
