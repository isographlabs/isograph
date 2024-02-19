import type {ReaderArtifact, ReaderAst} from '@isograph/react';
import { RepositoryDetail as resolver } from '../../../RepositoryDetail.tsx';
import PullRequestConnection__PullRequestTable, { PullRequestConnection__PullRequestTable__outputType} from '../../PullRequestConnection/PullRequestTable/reader';
import Repository__RepositoryLink, { Repository__RepositoryLink__outputType} from '../../Repository/RepositoryLink/reader';
import Starrable__IsStarred, { Starrable__IsStarred__outputType} from '../../Starrable/IsStarred/reader';

// the type, when read out (either via useLazyReference or via graph)
export type Query__RepositoryDetail__outputType = (React.FC<any>);

const readerAst: ReaderAst<Query__RepositoryDetail__param> = [
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
        kind: "Resolver",
        alias: "IsStarred",
        arguments: null,
        readerArtifact: Starrable__IsStarred,
        usedRefetchQueries: [],
      },
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
    IsStarred: Starrable__IsStarred__outputType,
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

const artifact: ReaderArtifact<
  Query__RepositoryDetail__param,
  Query__RepositoryDetail__param,
  Query__RepositoryDetail__outputType
> = {
  kind: "ReaderArtifact",
  resolver: resolver as any,
  readerAst,
  variant: { kind: "Component", componentName: "Query.RepositoryDetail" },
};

export default artifact;
