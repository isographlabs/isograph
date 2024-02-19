import type {ReaderArtifact, ReaderAst} from '@isograph/react';
import { CommentList as resolver } from '../../../CommentList.tsx';
import IssueComment__formattedCommentCreationDate, { IssueComment__formattedCommentCreationDate__outputType} from '../../IssueComment/formattedCommentCreationDate/reader';

// the type, when read out (either via useLazyReference or via graph)
export type PullRequest__CommentList__outputType = (React.FC<any>);

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

export type PullRequest__CommentList__param = { data:
{
  comments: {
    edges: (({
      node: ({
        id: string,
        bodyText: string,
        formattedCommentCreationDate: IssueComment__formattedCommentCreationDate__outputType,
        author: ({
          login: string,
        } | null),
      } | null),
    } | null))[],
  },
},
[index: string]: any };

const artifact: ReaderArtifact<
  PullRequest__CommentList__param,
  PullRequest__CommentList__param,
  PullRequest__CommentList__outputType
> = {
  kind: "ReaderArtifact",
  resolver: resolver as any,
  readerAst,
  variant: { kind: "Component", componentName: "PullRequest.CommentList" },
};

export default artifact;
