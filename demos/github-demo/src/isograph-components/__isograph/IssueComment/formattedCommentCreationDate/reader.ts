import type {ReaderArtifact, ReaderAst} from '@isograph/react';
import { IssueComment__formattedCommentCreationDate__param } from './param_type.ts';
import { IssueComment__formattedCommentCreationDate__outputType } from './output_type.ts';
import { formattedCommentCreationDate as resolver } from '../../../CommentList.tsx';

const readerAst: ReaderAst<IssueComment__formattedCommentCreationDate__param> = [
  {
    kind: "Scalar",
    fieldName: "createdAt",
    alias: null,
    arguments: null,
  },
];

const artifact: ReaderArtifact<
  IssueComment__formattedCommentCreationDate__param,
  IssueComment__formattedCommentCreationDate__outputType
> = {
  kind: "ReaderArtifact",
  resolver: resolver as any,
  readerAst,
  variant: { kind: "Eager" },
};

export default artifact;
