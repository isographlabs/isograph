import type {IsographNonFetchableResolver, ReaderAst} from '@isograph/react';
const resolver = x => x;
import Query__home_page_list, { ReadOutType as Query__home_page_list__outputType } from './home_page_list.isograph';
import UserStatus____refetch, { ReadOutType as UserStatus____refetch__outputType } from '../UserStatus/__refetch.isograph';

// the type, when read out (either via useLazyReference or via graph)
export type ReadOutType = ResolverReturnType;

// TODO support changing this
export type ReadFromStoreType = ResolverParameterType;

const readerAst: ReaderAst<ReadFromStoreType> = [
  {
    kind: "Resolver",
    alias: "home_page_list",
    arguments: null,
    resolver: Query__home_page_list,
    variant: "Component",
    usedRefetchQueries: [0, ],
  },
  {
    kind: "Linked",
    fieldName: "viewer",
    alias: null,
    arguments: null,
    selections: [
      {
        kind: "Linked",
        fieldName: "status",
        alias: null,
        arguments: null,
        selections: [
          {
            kind: "Scalar",
            fieldName: "emoji",
            alias: null,
            arguments: null,
          },
          {
            kind: "RefetchField",
            alias: "__refetch",
            resolver: UserStatus____refetch,
            refetchQuery: 1,
          },
        ],
      },
    ],
  },
];

export type ResolverParameterType = {
  home_page_list: Query__home_page_list__outputType,
  viewer: {
    status: ({
      emoji: (string | null),
      __refetch: UserStatus____refetch__outputType,
    } | null),
  },
};

// The type, when returned from the resolver
export type ResolverReturnType = ResolverParameterType;

const artifact: IsographNonFetchableResolver<ReadFromStoreType, ResolverParameterType, ReadOutType> = {
  kind: 'NonFetchableResolver',
  resolver: resolver as any,
  readerAst,
};

export default artifact;
