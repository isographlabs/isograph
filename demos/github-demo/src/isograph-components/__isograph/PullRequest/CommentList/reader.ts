import type {ReaderArtifact, ReaderAst} from '@isograph/react';
import { PullRequest__CommentList__param } from './param_type.ts';
import { PullRequest__CommentList__outputType } from './output_type.ts';
import { CommentList as resolver } from '../../../CommentList.tsx';
import IssueComment__formattedCommentCreationDate from '../../IssueComment/formattedCommentCreationDate/reader';

const readerAst: ReaderAst<PullRequest__CommentList__param> = [
  {
    kind: "Linked",
    fieldName: "comments",
    alias: null,
    arguments: [
      [
        "last",
        { kind: "Variable", name: "last" },
      ],
    ],
    selections: [
      {
        kind: "Linked",
        fieldName: "edges",
        alias: null,
        arguments: null,
        selections: [
          {
            kind: "Linked",
            fieldName: "node",
            alias: null,
            arguments: null,
            selections: [
              {
                kind: "Scalar",
                fieldName: "id",
                alias: null,
                arguments: null,
              },
              {
                kind: "Scalar",
                fieldName: "bodyText",
                alias: null,
                arguments: null,
              },
              {
                kind: "Resolver",
                alias: "formattedCommentCreationDate",
                arguments: null,
                readerArtifact: IssueComment__formattedCommentCreationDate,
                usedRefetchQueries: [],
              },
              {
                kind: "Linked",
                fieldName: "author",
                alias: null,
                arguments: null,
                selections: [
                  {
                    kind: "Scalar",
                    fieldName: "login",
                    alias: null,
                    arguments: null,
                  },
                ],
              },
            ],
          },
        ],
      },
    ],
  },
];

const artifact: ReaderArtifact<
  PullRequest__CommentList__param,
  PullRequest__CommentList__outputType
> = {
  kind: "ReaderArtifact",
  fieldName: "CommentList",
  resolver: resolver as any,
  readerAst,
  variant: { kind: "Component", componentName: "PullRequest.CommentList" },
};

export default artifact;
