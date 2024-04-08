import type {ReaderArtifact, ReaderAst} from '@isograph/react';
import { PullRequest__createdAtFormatted__param } from './param_type.ts';
import { PullRequest__createdAtFormatted__outputType } from './output_type.ts';
import { createdAtFormatted as resolver } from '../../../PullRequestTable.tsx';

const readerAst: ReaderAst<PullRequest__createdAtFormatted__param> = [
  {
    kind: "Scalar",
    fieldName: "createdAt",
    alias: null,
    arguments: null,
  },
];

const artifact: ReaderArtifact<
  PullRequest__createdAtFormatted__param,
  PullRequest__createdAtFormatted__outputType
> = {
  kind: "ReaderArtifact",
  resolver: resolver as any,
  readerAst,
  variant: { kind: "Eager" },
};

export default artifact;
