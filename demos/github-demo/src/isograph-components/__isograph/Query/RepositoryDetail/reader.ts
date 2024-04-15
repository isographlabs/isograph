import type {ReaderArtifact, ReaderAst} from '@isograph/react';
import { Query__RepositoryDetail__param } from './param_type.ts';
import { Query__RepositoryDetail__outputType } from './output_type.ts';
import { RepositoryDetail as resolver } from '../../../RepositoryDetail.tsx';
import PullRequestConnection__PullRequestTable from '../../PullRequestConnection/PullRequestTable/reader';
import Repository__RepositoryLink from '../../Repository/RepositoryLink/reader';
import Starrable__IsStarred from '../../Starrable/IsStarred/reader';

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

const artifact: ReaderArtifact<
  Query__RepositoryDetail__param,
  Query__RepositoryDetail__outputType
> = {
  kind: "ReaderArtifact",
  resolver: resolver as any,
  readerAst,
  variant: { kind: "Component", componentName: "Query.RepositoryDetail" },
};

export default artifact;
