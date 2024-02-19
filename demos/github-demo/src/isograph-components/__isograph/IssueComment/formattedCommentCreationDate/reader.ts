import type {ReaderArtifact, ReaderAst} from '@isograph/react';
import { formattedCommentCreationDate as resolver } from '../../../CommentList.tsx';

// the type, when read out (either via useLazyReference or via graph)
export type IssueComment__formattedCommentCreationDate__outputType = ReturnType<typeof resolver>;

const readerAst: ReaderAst<IssueComment__formattedCommentCreationDate__param> = [
  {
    kind: "Scalar",
    fieldName: "createdAt",
    alias: null,
    arguments: null,
  },
];

export type IssueComment__formattedCommentCreationDate__param = {
  createdAt: string,
};

const artifact: ReaderArtifact<
  IssueComment__formattedCommentCreationDate__param,
  IssueComment__formattedCommentCreationDate__param,
  IssueComment__formattedCommentCreationDate__outputType
> = {
  kind: "ReaderArtifact",
  resolver: resolver as any,
  readerAst,
  variant: { kind: "Eager" },
};

export default artifact;
