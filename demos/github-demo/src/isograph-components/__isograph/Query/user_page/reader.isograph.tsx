import type {IsographNonFetchableResolver, ReaderAst} from '@isograph/react';
const resolver = (x: any) => x;
import Query__header, { ReadOutType as Query__header__outputType } from '../header/reader.isograph';
import Query__user_detail, { ReadOutType as Query__user_detail__outputType } from '../user_detail/reader.isograph';

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
    alias: "user_detail",
    arguments: null,
    resolver: Query__user_detail,
    variant: "Component",
    usedRefetchQueries: [],
  },
];

export type ResolverParameterType = {
  header: Query__header__outputType,
  user_detail: Query__user_detail__outputType,
};

// The type, when returned from the resolver
export type ResolverReturnType = ResolverParameterType;

const artifact: IsographNonFetchableResolver<ReadFromStoreType, ResolverParameterType, ReadOutType> = {
  kind: 'NonFetchableResolver',
  resolver: resolver as any,
  readerAst,
};

export default artifact;
