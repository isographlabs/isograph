import type {BoultonFetchableResolver, ReaderAst, FragmentReference} from '@boulton/react';
import { getRefRendererForName } from '@boulton/react';
const resolver = x => x;
import Query__header, { ReadOutType as Query__header__outputType } from './Query__header.boulton';
import Query__repository_detail, { ReadOutType as Query__repository_detail__outputType } from './Query__repository_detail.boulton';

const queryText = 'query repository_page ($repositoryName: String!, $repositoryOwner: String!, $first: Int!) {\
  repository__name_repositoryName__owner_repositoryOwner: repository(name: $repositoryName, owner: $repositoryOwner) {\
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
    pullRequests__last_first: pullRequests(last: $first) {\
      edges {\
        node {\
          closed,\
          createdAt,\
          id,\
          number,\
          title,\
          totalCommentsCount,\
          author {\
            login,\
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

const artifact: BoultonFetchableResolver<ReadFromStoreType, ResolverParameterType, ReadOutType> = {
  kind: 'FetchableResolver',
  queryText,
  normalizationAst,
  readerAst,
  resolver: resolver as any,
  convert: ((resolver, data) => resolver(data)),
};

export default artifact;
