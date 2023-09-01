import type {IsographFetchableResolver, ReaderAst, FragmentReference, NormalizationAst} from '@isograph/react';
const resolver = x => x;
import Query__header, { ReadOutType as Query__header__outputType } from './header.isograph';
import Query__home_page_list, { ReadOutType as Query__home_page_list__outputType } from './home_page_list.isograph';

import rf1 from '../Query/home_page/__refetchQuery.isograph'
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
    arguments: null,
    selections: [
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
    usedRefetchQueries: [0],
    // This should only exist on refetch queries
    refetchQuery: 0,
  },
  {
    kind: "Resolver",
    alias: "home_page_list",
    arguments: null,
    resolver: Query__home_page_list,
    variant: "Component",
    usedRefetchQueries: [0],
    // This should only exist on refetch queries
    refetchQuery: 0,
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
  nestedRefetchQueries: [rf1],
};

export default artifact;
