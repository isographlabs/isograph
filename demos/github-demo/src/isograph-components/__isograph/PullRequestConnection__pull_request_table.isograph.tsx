import type {IsographNonFetchableResolver, ReaderAst} from '@isograph/react';
import { pull_request_table as resolver } from '../pull_request_table.tsx';
import Actor__user_link, { ReadOutType as Actor__user_link__outputType } from './Actor__user_link.isograph';
import PullRequest__created_at_formatted, { ReadOutType as PullRequest__created_at_formatted__outputType } from './PullRequest__created_at_formatted.isograph';
import PullRequest__pull_request_link, { ReadOutType as PullRequest__pull_request_link__outputType } from './PullRequest__pull_request_link.isograph';

// the type, when read out (either via useLazyReference or via graph)
export type ReadOutType = (additionalRuntimeProps: Object | void) => (React.ReactElement<any, any> | null);

// TODO support changing this
export type ReadFromStoreType = ResolverParameterType;

const readerAst: ReaderAst<ReadFromStoreType> = [
  {
    kind: "Linked",
    response_name: "edges",
    alias: null,
    arguments: null,
    selections: [
      {
        kind: "Linked",
        response_name: "node",
        alias: null,
        arguments: null,
        selections: [
          {
            kind: "Resolver",
            alias: "pull_request_link",
            arguments: null,
            resolver: PullRequest__pull_request_link,
            variant: "Component",
          },
          {
            kind: "Scalar",
            response_name: "number",
            alias: null,
            arguments: null,
          },
          {
            kind: "Scalar",
            response_name: "id",
            alias: null,
            arguments: null,
          },
          {
            kind: "Scalar",
            response_name: "title",
            alias: null,
            arguments: null,
          },
          {
            kind: "Linked",
            response_name: "author",
            alias: null,
            arguments: null,
            selections: [
              {
                kind: "Resolver",
                alias: "user_link",
                arguments: null,
                resolver: Actor__user_link,
                variant: "Component",
              },
              {
                kind: "Scalar",
                response_name: "login",
                alias: null,
                arguments: null,
              },
            ],
          },
          {
            kind: "Scalar",
            response_name: "closed",
            alias: null,
            arguments: null,
          },
          {
            kind: "Scalar",
            response_name: "totalCommentsCount",
            alias: null,
            arguments: null,
          },
          {
            kind: "Resolver",
            alias: "created_at_formatted",
            arguments: null,
            resolver: PullRequest__created_at_formatted,
            variant: "Eager",
          },
        ],
      },
    ],
  },
];

export type ResolverParameterType = { data:
{
  edges: (({
    node: ({
      pull_request_link: PullRequest__pull_request_link__outputType,
      number: number,
      id: string,
      title: string,
      author: ({
        user_link: Actor__user_link__outputType,
        login: string,
      } | null),
      closed: boolean,
      totalCommentsCount: (number | null),
      created_at_formatted: PullRequest__created_at_formatted__outputType,
    } | null),
  } | null))[],
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
