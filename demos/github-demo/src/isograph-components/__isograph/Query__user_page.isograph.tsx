import type {IsographFetchableResolver, ReaderAst, FragmentReference} from '@isograph/react';
import { getRefRendererForName } from '@isograph/react';
const resolver = x => x;
import Query__header, { ReadOutType as Query__header__outputType } from './Query__header.isograph';
import Query__user_detail, { ReadOutType as Query__user_detail__outputType } from './Query__user_detail.isograph';

const queryText = 'query user_page ($first: Int!, $userLogin: String!) {\
  user__login_userLogin: user(login: $userLogin) {\
    id,\
    name,\
    repositories__last_first: repositories(last: $first) {\
      edges {\
        node {\
          description,\
          forkCount,\
          id,\
          name,\
          nameWithOwner,\
          stargazerCount,\
          owner {\
            id,\
            login,\
          },\
          pullRequests__first_first: pullRequests(first: $first) {\
            totalCount,\
          },\
          watchers__first_first: watchers(first: $first) {\
            totalCount,\
          },\
        },\
      },\
    },\
  },\
  viewer {\
    avatarUrl,\
    id,\
    name,\
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
