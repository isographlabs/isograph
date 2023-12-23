import type {ReaderArtifact, ReaderAst} from '@isograph/react';
import { formatted_comment_creation_date as resolver } from '../../../isograph-components/comment_list.tsx';

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

const artifact: ReaderArtifact<ReadFromStoreType, ResolverParameterType, ReadOutType> = {
  kind: "ReaderArtifact",
  resolver: resolver as any,
  readerAst,
  variant: { kind: "Eager" },
};

export default artifact;
