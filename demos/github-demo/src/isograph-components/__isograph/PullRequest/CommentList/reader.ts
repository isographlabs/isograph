import type {ReaderArtifact, ReaderAst} from '@isograph/react';
import { CommentList as resolver } from '../../../CommentList.tsx';
import IssueComment__formattedCommentCreationDate, { ReadOutType as IssueComment__formattedCommentCreationDate__outputType } from '../../IssueComment/formattedCommentCreationDate/reader';

// the type, when read out (either via useLazyReference or via graph)
export type ReadOutType = (React.FC<any>);

export type ReadFromStoreType = ResolverParameterType;

const readerAst: ReaderAst<ReadFromStoreType> = [
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

export type ResolverParameterType = { data:
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

// The type, when returned from the resolver
export type ResolverReturnType = ReturnType<typeof resolver>;

const artifact: ReaderArtifact<ReadFromStoreType, ResolverParameterType, ReadOutType> = {
  kind: "ReaderArtifact",
  resolver: resolver as any,
  readerAst,
  variant: { kind: "Component", componentName: "PullRequest.CommentList" },
};

export default artifact;
