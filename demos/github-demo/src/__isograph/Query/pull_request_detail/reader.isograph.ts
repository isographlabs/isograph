import type {ReaderArtifact, ReaderAst} from '@isograph/react';
import { pull_request_detail as resolver } from '../../../isograph-components/pull_request_detail.tsx';
import PullRequest__comment_list, { ReadOutType as PullRequest__comment_list__outputType } from '../../PullRequest/comment_list/reader.isograph';

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
            alias: "comment_list",
            arguments: null,
            readerArtifact: PullRequest__comment_list,
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
      comment_list: PullRequest__comment_list__outputType,
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
  variant: { kind: "Component", componentName: "Query.pull_request_detail" },
};

export default artifact;
