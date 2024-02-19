import type {ReaderArtifact, ReaderAst} from '@isograph/react';
import { PullRequestDetail as resolver } from '../../../PullRequestDetail.tsx';
import PullRequest__CommentList, { ReadOutType as PullRequest__CommentList__outputType } from '../../PullRequest/CommentList/reader';

// the type, when read out (either via useLazyReference or via graph)
export type ReadOutType = (React.FC<any>);

const readerAst: ReaderAst<Query__PullRequestDetail__param> = [
  {
    kind: "Linked",
    fieldName: "repository",
    alias: null,
    arguments: [
      [
        "owner",
        { kind: "Variable", name: "repositoryOwner" },
      ],

      [
        "name",
        { kind: "Variable", name: "repositoryName" },
      ],
    ],
    selections: [
      {
        kind: "Linked",
        fieldName: "pullRequest",
        alias: null,
        arguments: [
          [
            "number",
            { kind: "Variable", name: "pullRequestNumber" },
          ],
        ],
        selections: [
          {
            kind: "Scalar",
            fieldName: "title",
            alias: null,
            arguments: null,
          },
          {
            kind: "Scalar",
            fieldName: "bodyHTML",
            alias: null,
            arguments: null,
          },
          {
            kind: "Resolver",
            alias: "CommentList",
            arguments: null,
            readerArtifact: PullRequest__CommentList,
            usedRefetchQueries: [],
          },
        ],
      },
    ],
  },
];

export type Query__PullRequestDetail__param = { data:
{
  repository: ({
    pullRequest: ({
      title: string,
      bodyHTML: string,
      CommentList: PullRequest__CommentList__outputType,
    } | null),
  } | null),
},
[index: string]: any };

const artifact: ReaderArtifact<
  Query__PullRequestDetail__param,
  Query__PullRequestDetail__param,
  ReadOutType
> = {
  kind: "ReaderArtifact",
  resolver: resolver as any,
  readerAst,
  variant: { kind: "Component", componentName: "Query.PullRequestDetail" },
};

export default artifact;
