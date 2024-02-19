import type {ReaderArtifact, ReaderAst} from '@isograph/react';
import { createdAtFormatted as resolver } from '../../../PullRequestTable.tsx';

// the type, when read out (either via useLazyReference or via graph)
export type ReadOutType = ReturnType<typeof resolver>;

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

const artifact: ReaderArtifact<ReadFromStoreType, PullRequest__createdAtFormatted__param, ReadOutType> = {
  kind: "ReaderArtifact",
  resolver: resolver as any,
  readerAst,
  variant: { kind: "Eager" },
};

export default artifact;
