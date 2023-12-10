import type {IsographNonFetchableResolver, ReaderAst} from '@isograph/react';
import { home_page_list as resolver } from '../../../home_page_list.tsx';
import User____refetch, { ReadOutType as User____refetch__outputType } from '../../User/__refetch/reader.isograph';
import User__repository_list, { ReadOutType as User__repository_list__outputType } from '../../User/repository_list/reader.isograph';

// the type, when read out (either via useLazyReference or via graph)
export type ReadOutType = (additionalRuntimeProps: Object | void) => (React.ReactElement<any, any> | null);

export type ReadFromStoreType = ResolverParameterType;

const readerAst: ReaderAst<ReadFromStoreType> = [
  {
    kind: "Linked",
    fieldName: "viewer",
    alias: null,
    arguments: null,
    selections: [
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
        kind: "Resolver",
        alias: "repository_list",
        arguments: null,
        resolver: User__repository_list,
        variant: "Component",
        usedRefetchQueries: [],
      },
      {
        kind: "RefetchField",
        alias: "__refetch",
        resolver: User____refetch,
        refetchQuery: 0,
      },
    ],
  },
];

export type ResolverParameterType = { data:
{
  viewer: {
    login: string,
    name: (string | null),
    repository_list: User__repository_list__outputType,
    __refetch: User____refetch__outputType,
  },
},
[index: string]: any };

// The type, when returned from the resolver
export type ResolverReturnType = ReturnType<typeof resolver>;

const artifact: IsographNonFetchableResolver<ReadFromStoreType, ResolverParameterType, ReadOutType> = {
  kind: 'NonFetchableResolver',
  resolver: resolver as any,
  readerAst,
};

export default artifact;
