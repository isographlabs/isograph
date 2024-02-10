import type {ReaderArtifact, ReaderAst} from '@isograph/react';
import { makeNetworkRequest } from '@isograph/react';
const resolver = (environment, artifact, variables) => () => makeNetworkRequest(environment, artifact, variables);

// the type, when read out (either via useLazyReference or via graph)
export type ReadOutType = any;

export type ReadFromStoreType = User____refetch__param;

const readerAst: ReaderAst<ReadFromStoreType> = [
  {
    kind: "Scalar",
    fieldName: "id",
    alias: null,
    arguments: null,
  },
];

export type User____refetch__param = {
  id: string,
};

// The type, when returned from the resolver
export type ResolverReturnType = ReturnType<typeof resolver>;

const artifact: ReaderArtifact<ReadFromStoreType, User____refetch__param, ReadOutType> = {
  kind: "ReaderArtifact",
  resolver: resolver as any,
  readerAst,
  variant: { kind: "Eager" },
};

export default artifact;
