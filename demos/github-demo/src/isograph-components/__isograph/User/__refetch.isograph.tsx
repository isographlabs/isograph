import type {IsographNonFetchableResolver, ReaderAst} from '@isograph/react';
import { makeNetworkRequest } from '@isograph/react';
// const resolver = makeRefetchableFieldResolver(artifact);
const resolver = () => (artifact, variables) => makeNetworkRequest(artifact, variables);

// the type, when read out (either via useLazyReference or via graph)
export type ReadOutType = any;

// TODO support changing this
export type ReadFromStoreType = ResolverParameterType;

const readerAst: ReaderAst<ReadFromStoreType> = [
  {
    kind: "Scalar",
    fieldName: "id",
    alias: null,
    arguments: null,
  },
];

export type ResolverParameterType = {
  id: string,
};

// The type, when returned from the resolver
export type ResolverReturnType = ReturnType<typeof resolver>;

const artifact: IsographNonFetchableResolver<ReadFromStoreType, ResolverParameterType, ReadOutType> = {
  kind: 'NonFetchableResolver',
  resolver: resolver as any,
  readerAst,
};

export default artifact;
