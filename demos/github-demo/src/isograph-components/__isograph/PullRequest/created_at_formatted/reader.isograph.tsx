import type {IsographNonFetchableResolver, ReaderAst} from '@isograph/react';
import { created_at_formatted as resolver } from '../../../pull_request_table.tsx';

// the type, when read out (either via useLazyReference or via graph)
export type ReadOutType = ResolverReturnType;

export type ReadFromStoreType = ResolverParameterType;

const readerAst: ReaderAst<ReadFromStoreType> = [
  {
    kind: "Scalar",
    fieldName: "createdAt",
    alias: null,
    arguments: null,
  },
];

export type ResolverParameterType = {
  createdAt: string,
};

// The type, when returned from the resolver
export type ResolverReturnType = ReturnType<typeof resolver>;

const artifact: IsographNonFetchableResolver<ReadFromStoreType, ResolverParameterType, ReadOutType> = {
  kind: 'NonFetchableResolver',
  resolver: resolver as any,
  readerAst,
};

export default artifact;
