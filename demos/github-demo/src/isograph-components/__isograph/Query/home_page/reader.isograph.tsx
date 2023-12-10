import type {IsographNonFetchableResolver, ReaderAst} from '@isograph/react';
const resolver = (x: any) => x;
import Query__header, { ReadOutType as Query__header__outputType } from '../header/reader.isograph';
import Query__home_page_list_wrapper, { ReadOutType as Query__home_page_list_wrapper__outputType } from '../home_page_list_wrapper/reader.isograph';

// the type, when read out (either via useLazyReference or via graph)
export type ReadOutType = ResolverReturnType;

export type ReadFromStoreType = ResolverParameterType;

const readerAst: ReaderAst<ReadFromStoreType> = [
  {
    kind: "Resolver",
    alias: "header",
    arguments: null,
    resolver: Query__header,
    variant: "Component",
    usedRefetchQueries: [],
  },
  {
    kind: "Resolver",
    alias: "home_page_list_wrapper",
    arguments: null,
    resolver: Query__home_page_list_wrapper,
    variant: "Eager",
    usedRefetchQueries: [0, 1, ],
  },
];

export type ResolverParameterType = {
  header: Query__header__outputType,
  home_page_list_wrapper: Query__home_page_list_wrapper__outputType,
};

// The type, when returned from the resolver
export type ResolverReturnType = ResolverParameterType;

const artifact: IsographNonFetchableResolver<ReadFromStoreType, ResolverParameterType, ReadOutType> = {
  kind: 'NonFetchableResolver',
  resolver: resolver as any,
  readerAst,
};

export default artifact;
