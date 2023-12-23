import type {ReaderArtifact, ReaderAst} from '@isograph/react';
import { pull_request_table as resolver } from '../../../isograph-components/pull_request_table.tsx';
import Actor__user_link, { ReadOutType as Actor__user_link__outputType } from '../../Actor/user_link/reader.isograph';
import PullRequest__created_at_formatted, { ReadOutType as PullRequest__created_at_formatted__outputType } from '../../PullRequest/created_at_formatted/reader.isograph';
import PullRequest__pull_request_link, { ReadOutType as PullRequest__pull_request_link__outputType } from '../../PullRequest/pull_request_link/reader.isograph';

// the type, when read out (either via useLazyReference or via graph)
export type ReadOutType = (additionalRuntimeProps: Object | void) => (React.ReactElement<any, any> | null);

export type ReadFromStoreType = ResolverParameterType;

const readerAst: ReaderAst<ReadFromStoreType> = [
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
            kind: "Resolver",
            alias: "pull_request_link",
            arguments: null,
            readerArtifact: PullRequest__pull_request_link,
            usedRefetchQueries: [],
          },
          {
            kind: "Scalar",
            fieldName: "number",
            alias: null,
            arguments: null,
          },
          {
            kind: "Scalar",
            fieldName: "title",
            alias: null,
            arguments: null,
          },
          {
            kind: "Linked",
            fieldName: "author",
            alias: null,
            arguments: null,
            selections: [
              {
                kind: "Resolver",
                alias: "user_link",
                arguments: null,
                readerArtifact: Actor__user_link,
                usedRefetchQueries: [],
              },
              {
                kind: "Scalar",
                fieldName: "login",
                alias: null,
                arguments: null,
              },
            ],
          },
          {
            kind: "Scalar",
            fieldName: "closed",
            alias: null,
            arguments: null,
          },
          {
            kind: "Scalar",
            fieldName: "totalCommentsCount",
            alias: null,
            arguments: null,
          },
          {
            kind: "Resolver",
            alias: "created_at_formatted",
            arguments: null,
            readerArtifact: PullRequest__created_at_formatted,
            usedRefetchQueries: [],
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
      id: string,
      pull_request_link: PullRequest__pull_request_link__outputType,
      number: number,
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

const artifact: ReaderArtifact<ReadFromStoreType, ResolverParameterType, ReadOutType> = {
  kind: "ReaderArtifact",
  resolver: resolver as any,
  readerAst,
  variant: { kind: "Component", componentName: "PullRequestConnection.pull_request_table" },
};

export default artifact;
