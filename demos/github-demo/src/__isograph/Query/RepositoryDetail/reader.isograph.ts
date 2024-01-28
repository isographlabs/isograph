import type {ReaderArtifact, ReaderAst} from '@isograph/react';
import { RepositoryDetail as resolver } from '../../../isograph-components/RepositoryDetail.tsx';
import PullRequestConnection__PullRequestTable, { ReadOutType as PullRequestConnection__PullRequestTable__outputType } from '../../PullRequestConnection/PullRequestTable/reader.isograph';
import Repository__RepositoryLink, { ReadOutType as Repository__RepositoryLink__outputType } from '../../Repository/RepositoryLink/reader.isograph';

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
            alias: "RepositoryLink",
            arguments: null,
            readerArtifact: Repository__RepositoryLink,
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
            alias: "PullRequestTable",
            arguments: null,
            readerArtifact: PullRequestConnection__PullRequestTable,
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
      RepositoryLink: Repository__RepositoryLink__outputType,
      nameWithOwner: string,
    } | null),
    pullRequests: {
      PullRequestTable: PullRequestConnection__PullRequestTable__outputType,
    },
  } | null),
},
[index: string]: any };

// The type, when returned from the resolver
export type ResolverReturnType = ReturnType<typeof resolver>;

const artifact: ReaderArtifact<ReadFromStoreType, ResolverParameterType, ReadOutType> = {
  kind: "ReaderArtifact",
  resolver: resolver as any,
  readerAst,
  variant: { kind: "Component", componentName: "Query.RepositoryDetail" },
};

export default artifact;
