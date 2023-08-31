import type {IsographNonFetchableResolver, ReaderAst} from '@isograph/react';
import { pull_request_detail as resolver } from '../../pull_request_detail.tsx';
import PullRequest__comment_list, { ReadOutType as PullRequest__comment_list__outputType } from '../PullRequest/comment_list.isograph';

// the type, when read out (either via useLazyReference or via graph)
export type ReadOutType = (additionalRuntimeProps: Object | void) => (React.ReactElement<any, any> | null);

// TODO support changing this
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
            resolver: PullRequest__comment_list,
            variant: "Component",
            usedRefetchQueries: [0],
            // This should only exist on refetch queries
            refetchQuery: 0,
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

const artifact: IsographNonFetchableResolver<ReadFromStoreType, ResolverParameterType, ReadOutType> = {
  kind: 'NonFetchableResolver',
  resolver: resolver as any,
  readerAst,
};

export default artifact;
