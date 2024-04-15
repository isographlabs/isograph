import type {ReaderArtifact, ReaderAst} from '@isograph/react';
import { Query__PullRequestDetail__param } from './param_type.ts';
import { Query__PullRequestDetail__outputType } from './output_type.ts';
import { PullRequestDetail as resolver } from '../../../PullRequestDetail.tsx';
import PullRequest__CommentList from '../../PullRequest/CommentList/reader';

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

const artifact: ReaderArtifact<
  Query__PullRequestDetail__param,
  Query__PullRequestDetail__outputType
> = {
  kind: "ReaderArtifact",
  resolver: resolver as any,
  readerAst,
  variant: { kind: "Component", componentName: "Query.PullRequestDetail" },
};

export default artifact;
