import type {ReaderArtifact, ReaderAst} from '@isograph/react';
import { PullRequestDetail as resolver } from '../../../isograph-components/pull_request_detail.tsx';
import PullRequest__CommentList, { ReadOutType as PullRequest__CommentList__outputType } from '../../PullRequest/CommentList/reader.isograph';

// the type, when read out (either via useLazyReference or via graph)
export type ReadOutType = (React.FC<any>);

export type ReadFromStoreType = ResolverParameterType;

const readerAst: ReaderAst<ReadFromStoreType> = [
  {
    kind: "Linked",
    fieldName: "repository",
    alias: null,
    arguments: [
      {
        argumentName: "owner",
        variableName: "repositoryOwner",
      },

      {
        argumentName: "name",
        variableName: "repositoryName",
      },
    ],
    selections: [
      {
        kind: "Linked",
        fieldName: "pullRequest",
        alias: null,
        arguments: [
          {
            argumentName: "number",
            variableName: "pullRequestNumber",
          },
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

export type ResolverParameterType = { data:
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

// The type, when returned from the resolver
export type ResolverReturnType = ReturnType<typeof resolver>;

const artifact: ReaderArtifact<ReadFromStoreType, ResolverParameterType, ReadOutType> = {
  kind: "ReaderArtifact",
  resolver: resolver as any,
  readerAst,
  variant: { kind: "Component", componentName: "Query.PullRequestDetail" },
};

export default artifact;
