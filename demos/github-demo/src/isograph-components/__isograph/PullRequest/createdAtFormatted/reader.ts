import type {ReaderArtifact, ReaderAst} from '@isograph/react';
import { createdAtFormatted as resolver } from '../../../PullRequestTable.tsx';

// the type, when read out (either via useLazyReference or via graph)
export type PullRequest__createdAtFormatted__outputType = ReturnType<typeof resolver>;

const readerAst: ReaderAst<PullRequest__createdAtFormatted__param> = [
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

const artifact: ReaderArtifact<
  PullRequest__createdAtFormatted__param,
  PullRequest__createdAtFormatted__param,
  PullRequest__createdAtFormatted__outputType
> = {
  kind: "ReaderArtifact",
  resolver: resolver as any,
  readerAst,
  variant: { kind: "Eager" },
};

export default artifact;
