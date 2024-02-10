import type {ReaderArtifact, ReaderAst} from '@isograph/react';
import { createdAtFormatted as resolver } from '../../../PullRequestTable.tsx';

// the type, when read out (either via useLazyReference or via graph)
export type ReadOutType = ResolverReturnType;

export type ReadFromStoreType = PullRequest__createdAtFormatted__param;

const readerAst: ReaderAst<ReadFromStoreType> = [
  {
    kind: "Scalar",
    fieldName: "createdAt",
    alias: null,
    arguments: null,
  },
];

export type PullRequest__createdAtFormatted__param = {
  createdAt: string,
};

// The type, when returned from the resolver
export type ResolverReturnType = ReturnType<typeof resolver>;

const artifact: ReaderArtifact<ReadFromStoreType, PullRequest__createdAtFormatted__param, ReadOutType> = {
  kind: "ReaderArtifact",
  resolver: resolver as any,
  readerAst,
  variant: { kind: "Eager" },
};

export default artifact;
