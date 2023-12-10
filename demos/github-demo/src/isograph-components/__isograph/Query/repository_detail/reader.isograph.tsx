import type {IsographNonFetchableResolver, ReaderAst} from '@isograph/react';
import { repository_detail as resolver } from '../../../repository_detail.tsx';
import PullRequestConnection__pull_request_table, { ReadOutType as PullRequestConnection__pull_request_table__outputType } from '../../PullRequestConnection/pull_request_table/reader.isograph';
import Repository__repository_link, { ReadOutType as Repository__repository_link__outputType } from '../../Repository/repository_link/reader.isograph';

// the type, when read out (either via useLazyReference or via graph)
export type ReadOutType = (additionalRuntimeProps: Object | void) => (React.ReactElement<any, any> | null);

export type ReadFromStoreType = ResolverParameterType;

const readerAst: ReaderAst<ReadFromStoreType> = [
  {
    kind: "Linked",
    fieldName: "repository",
    alias: null,
    arguments: [
      {
        argumentName: "name",
        variableName: "repositoryName",
      },

      {
        argumentName: "owner",
        variableName: "repositoryOwner",
      },
    ],
    selections: [
      {
        kind: "Scalar",
        fieldName: "nameWithOwner",
        alias: null,
        arguments: null,
      },
      {
        kind: "Linked",
        fieldName: "parent",
        alias: null,
        arguments: null,
        selections: [
          {
            kind: "Resolver",
            alias: "repository_link",
            arguments: null,
            resolver: Repository__repository_link,
            variant: "Component",
            usedRefetchQueries: [],
          },
          {
            kind: "Scalar",
            fieldName: "nameWithOwner",
            alias: null,
            arguments: null,
          },
        ],
      },
      {
        kind: "Linked",
        fieldName: "pullRequests",
        alias: null,
        arguments: [
          {
            argumentName: "last",
            variableName: "first",
          },
        ],
        selections: [
          {
            kind: "Resolver",
            alias: "pull_request_table",
            arguments: null,
            resolver: PullRequestConnection__pull_request_table,
            variant: "Component",
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
    nameWithOwner: string,
    parent: ({
      repository_link: Repository__repository_link__outputType,
      nameWithOwner: string,
    } | null),
    pullRequests: {
      pull_request_table: PullRequestConnection__pull_request_table__outputType,
    },
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
