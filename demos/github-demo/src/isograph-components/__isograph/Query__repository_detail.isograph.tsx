import type {IsographNonFetchableResolver, ReaderAst} from '@isograph/react';
import { repository_detail as resolver } from '../repository_detail.tsx';
import PullRequestConnection__pull_request_table, { ReadOutType as PullRequestConnection__pull_request_table__outputType } from './PullRequestConnection__pull_request_table.isograph';
import Repository__repository_link, { ReadOutType as Repository__repository_link__outputType } from './Repository__repository_link.isograph';

// the type, when read out (either via useLazyReference or via graph)
export type ReadOutType = (additionalRuntimeProps: Object | void) => (React.ReactElement<any, any> | null);

// TODO support changing this
export type ReadFromStoreType = ResolverParameterType;

const readerAst: ReaderAst<ReadFromStoreType> = [
  {
    kind: "Linked",
    response_name: "repository",
    alias: null,
    arguments: {
      "name": "repositoryName",
      "owner": "repositoryOwner",
    },
    selections: [
      {
        kind: "Scalar",
        response_name: "id",
        alias: null,
        arguments: null,
      },
      {
        kind: "Scalar",
        response_name: "nameWithOwner",
        alias: null,
        arguments: null,
      },
      {
        kind: "Linked",
        response_name: "parent",
        alias: null,
        arguments: null,
        selections: [
          {
            kind: "Resolver",
            alias: "repository_link",
            arguments: null,
            resolver: Repository__repository_link,
            variant: "Component",
          },
          {
            kind: "Scalar",
            response_name: "id",
            alias: null,
            arguments: null,
          },
          {
            kind: "Scalar",
            response_name: "nameWithOwner",
            alias: null,
            arguments: null,
          },
        ],
      },
      {
        kind: "Linked",
        response_name: "pullRequests",
        alias: null,
        arguments: {
          "last": "first",
        },
        selections: [
          {
            kind: "Resolver",
            alias: "pull_request_table",
            arguments: null,
            resolver: PullRequestConnection__pull_request_table,
            variant: "Component",
          },
        ],
      },
    ],
  },
];

export type ResolverParameterType = { data:
{
  repository: ({
    id: string,
    nameWithOwner: string,
    parent: ({
      repository_link: Repository__repository_link__outputType,
      id: string,
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
