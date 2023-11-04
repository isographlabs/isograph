import type {IsographNonFetchableResolver, ReaderAst} from '@isograph/react';
const resolver = (x: any) => x;
import Query__header, { ReadOutType as Query__header__outputType } from '../header/reader.isograph';
import Query__repository_detail, { ReadOutType as Query__repository_detail__outputType } from '../repository_detail/reader.isograph';

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
    alias: "repository_detail",
    arguments: null,
    resolver: Query__repository_detail,
    variant: "Component",
    usedRefetchQueries: [],
  },
];

export type ResolverParameterType = {
  header: Query__header__outputType,
  repository_detail: Query__repository_detail__outputType,
};

// The type, when returned from the resolver
export type ResolverReturnType = ResolverParameterType;

const artifact: IsographNonFetchableResolver<ReadFromStoreType, ResolverParameterType, ReadOutType> = {
  kind: 'NonFetchableResolver',
  resolver: resolver as any,
  readerAst,
};

export default artifact;
