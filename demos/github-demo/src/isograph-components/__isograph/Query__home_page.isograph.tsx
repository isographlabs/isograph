import type {IsographFetchableResolver, ReaderAst, FragmentReference} from '@isograph/react';
import { getRefRendererForName } from '@isograph/react';
const resolver = x => x;
import Query__header, { ReadOutType as Query__header__outputType } from './Query__header.isograph';
import Query__home_page_list, { ReadOutType as Query__home_page_list__outputType } from './Query__home_page_list.isograph';

const queryText = 'query home_page ($first: Int!) {\
  viewer {\
    id,\
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
          id,\
          name,\
          nameWithOwner,\
          stargazerCount,\
          owner {\
            id,\
            id,\
            login,\
          },\
          pullRequests____first___first: pullRequests(first: $first) {\
            totalCount,\
          },\
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

const normalizationAst = {notNeededForDemo: true};
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
