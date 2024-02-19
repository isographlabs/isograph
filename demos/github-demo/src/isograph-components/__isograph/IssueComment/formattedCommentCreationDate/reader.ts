import type {ReaderArtifact, ReaderAst} from '@isograph/react';
import { formattedCommentCreationDate as resolver } from '../../../CommentList.tsx';

// the type, when read out (either via useLazyReference or via graph)
export type ReadOutType = ReturnType<typeof resolver>;

export type ReadFromStoreType = IssueComment__formattedCommentCreationDate__param;

const readerAst: ReaderAst<ReadFromStoreType> = [
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

const artifact: ReaderArtifact<ReadFromStoreType, IssueComment__formattedCommentCreationDate__param, ReadOutType> = {
  kind: "ReaderArtifact",
  resolver: resolver as any,
  readerAst,
  variant: { kind: "Eager" },
};

export default artifact;
