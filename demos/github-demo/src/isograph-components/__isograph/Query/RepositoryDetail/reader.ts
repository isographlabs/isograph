import type {ReaderArtifact, ReaderAst} from '@isograph/react';
import { RepositoryDetail as resolver } from '../../../RepositoryDetail.tsx';
import PullRequestConnection__PullRequestTable, { ReadOutType as PullRequestConnection__PullRequestTable__outputType } from '../../PullRequestConnection/PullRequestTable/reader';
import Repository__RepositoryLink, { ReadOutType as Repository__RepositoryLink__outputType } from '../../Repository/RepositoryLink/reader';

// the type, when read out (either via useLazyReference or via graph)
export type ReadOutType = (React.FC<any>);

export type ReadFromStoreType = Query__RepositoryDetail__param;

const readerAst: ReaderAst<ReadFromStoreType> = [
  {
    kind: "Linked",
    fieldName: "repository",
    alias: null,
    arguments: [
      [
        "name",
        { kind: "Variable", name: "repositoryName" },
      ],

      [
        "owner",
        { kind: "Variable", name: "repositoryOwner" },
      ],
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
          [
            "last",
            { kind: "Variable", name: "first" },
          ],
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

export type Query__RepositoryDetail__param = { data:
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

const artifact: ReaderArtifact<ReadFromStoreType, Query__RepositoryDetail__param, ReadOutType> = {
  kind: "ReaderArtifact",
  resolver: resolver as any,
  readerAst,
  variant: { kind: "Component", componentName: "Query.RepositoryDetail" },
};

export default artifact;
