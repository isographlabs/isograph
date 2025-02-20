import type { EagerReaderArtifact, ReaderAst } from '@isograph/react';
import { PullRequest__createdAtFormatted__param } from './param_type';
import { PullRequest__createdAtFormatted__output_type } from './output_type';
import { createdAtFormatted as resolver } from '../../../PullRequestTable';

const readerAst: ReaderAst<PullRequest__createdAtFormatted__param> = [
  {
    kind: "Scalar",
    fieldName: "createdAt",
    alias: null,
    arguments: null,
    isUpdatable: false,
  },
];

const artifact: EagerReaderArtifact<
  PullRequest__createdAtFormatted__param,
  PullRequest__createdAtFormatted__output_type
> = {
  kind: "EagerReaderArtifact",
  fieldName: "PullRequest.createdAtFormatted",
  resolver,
  readerAst,
  hasUpdatable: false,
};

export default artifact;
