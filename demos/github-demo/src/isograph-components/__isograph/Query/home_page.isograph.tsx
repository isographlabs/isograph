import type {IsographFetchableResolver, ReaderAst, FragmentReference, NormalizationAst} from '@isograph/react';
import {getRefRendererForName} from '@isograph/react';
const resolver = x => x;
import Query__header, { ReadOutType as Query__header__outputType } from './header.isograph';
import Query__home_page_list, { ReadOutType as Query__home_page_list__outputType } from './home_page_list.isograph';

const queryText = 'query home_page ($first: Int!) {\
  viewer {\
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
  },\
}';

// TODO support changing this,
export type ReadFromStoreType = ResolverParameterType;

const normalizationAst: NormalizationAst = [
  {
    kind: "Linked",
    fieldName: "viewer",
    alias: null,
    arguments: null,
    selections: [
      {
        kind: "Scalar",
        fieldName: "avatarUrl",
        alias: null,
        arguments: null,
      },
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
      {
        kind: "Scalar",
        fieldName: "name",
        alias: null,
        arguments: null,
      },
      {
        kind: "Linked",
        fieldName: "repositories",
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
                    kind: "Scalar",
                    fieldName: "description",
                    alias: null,
                    arguments: null,
                  },
                  {
                    kind: "Scalar",
                    fieldName: "forkCount",
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
                  {
                    kind: "Linked",
                    fieldName: "pullRequests",
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
                        fieldName: "totalCount",
                        alias: null,
                        arguments: null,
                      },
                    ],
                  },
                  {
                    kind: "Scalar",
                    fieldName: "stargazerCount",
                    alias: null,
                    arguments: null,
                  },
                  {
                    kind: "Linked",
                    fieldName: "watchers",
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
                        fieldName: "totalCount",
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
    alias: "home_page_list",
    arguments: null,
    resolver: Query__home_page_list,
    variant: "Component",
  },
];

export type ResolverParameterType = {
  header: Query__header__outputType,
  home_page_list: Query__home_page_list__outputType,
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
