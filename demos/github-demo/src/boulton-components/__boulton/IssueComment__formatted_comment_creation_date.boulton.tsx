import type {BoultonNonFetchableResolver, ReaderAst} from '@boulton/react';
import { formatted_comment_creation_date as resolver } from '../comment_list.tsx';

// the type, when read out (either via useLazyReference or via graph)
export type ReadOutType = ResolverReturnType;

// TODO support changing this
export type ReadFromStoreType = ResolverParameterType;

const readerAst: ReaderAst<ReadFromStoreType> = [
  {
    kind: "Scalar",
    response_name: "createdAt",
    alias: null,
    arguments: null,
  },
];

export type ResolverParameterType = {
  createdAt: string,
};

// The type, when returned from the resolver
export type ResolverReturnType = ReturnType<typeof resolver>;

const artifact: BoultonNonFetchableResolver<ReadFromStoreType, ResolverParameterType, ReadOutType> = {
  kind: 'NonFetchableResolver',
  resolver: resolver as any,
  readerAst,
};

export default artifact;
