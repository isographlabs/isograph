import type { EagerReaderArtifact, ReaderAst } from '@isograph/react';
import { IssueComment__formattedCommentCreationDate__param } from './param_type';
import { IssueComment__formattedCommentCreationDate__output_type } from './output_type';
import { formattedCommentCreationDate as resolver } from '../../../CommentList';

const readerAst: ReaderAst<IssueComment__formattedCommentCreationDate__param> = [
  {
    kind: "Scalar",
    fieldName: "createdAt",
    alias: null,
    arguments: null,
  },
];

const artifact: EagerReaderArtifact<
  IssueComment__formattedCommentCreationDate__param,
  IssueComment__formattedCommentCreationDate__output_type
> = {
  kind: "EagerReaderArtifact",
  resolver,
  readerAst,
};

export default artifact;
