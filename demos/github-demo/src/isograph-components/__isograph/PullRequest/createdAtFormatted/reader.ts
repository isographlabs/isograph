import type {EagerReaderArtifact, ReaderAst, RefetchQueryNormalizationArtifact} from '@isograph/react';
import { PullRequest__createdAtFormatted__param } from './param_type';
import { PullRequest__createdAtFormatted__outputType } from './output_type';
import { createdAtFormatted as resolver } from '../../../PullRequestTable.tsx';

const readerAst: ReaderAst<PullRequest__createdAtFormatted__param> = [
  {
    kind: "Scalar",
    fieldName: "createdAt",
    alias: null,
    arguments: null,
  },
];

const artifact: EagerReaderArtifact<
  PullRequest__createdAtFormatted__param,
  PullRequest__createdAtFormatted__outputType
> = {
  kind: "EagerReaderArtifact",
  resolver,
  readerAst,
};

export default artifact;
