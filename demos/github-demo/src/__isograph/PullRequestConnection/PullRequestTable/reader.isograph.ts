import type {ReaderArtifact, ReaderAst} from '@isograph/react';
import { PullRequestTable as resolver } from '../../../isograph-components/pull_request_table.tsx';
import Actor__UserLink, { ReadOutType as Actor__UserLink__outputType } from '../../Actor/UserLink/reader.isograph';
import PullRequest__PullRequestLink, { ReadOutType as PullRequest__PullRequestLink__outputType } from '../../PullRequest/PullRequestLink/reader.isograph';
import PullRequest__created_at_formatted, { ReadOutType as PullRequest__created_at_formatted__outputType } from '../../PullRequest/created_at_formatted/reader.isograph';

// the type, when read out (either via useLazyReference or via graph)
export type ReadOutType = (React.FC<any>);

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
            alias: "PullRequestLink",
            arguments: null,
            readerArtifact: PullRequest__PullRequestLink,
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
                alias: "UserLink",
                arguments: null,
                readerArtifact: Actor__UserLink,
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
      PullRequestLink: PullRequest__PullRequestLink__outputType,
      number: number,
      title: string,
      author: ({
        UserLink: Actor__UserLink__outputType,
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
  variant: { kind: "Component", componentName: "PullRequestConnection.PullRequestTable" },
};

export default artifact;
