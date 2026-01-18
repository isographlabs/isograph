import type {ComponentReaderArtifact, ExtractSecondParam, ReaderAst } from '@isograph/react';
import { Query__RepositoryDetail__param } from './param_type';
import { RepositoryDetail as resolver } from '../../../RepositoryDetail';
import PullRequestConnection__PullRequestTable__resolver_reader from '../../PullRequestConnection/PullRequestTable/resolver_reader';
import Repository__IsStarred__resolver_reader from '../../Repository/IsStarred/resolver_reader';
import Repository__RepositoryLink__resolver_reader from '../../Repository/RepositoryLink/resolver_reader';

const readerAst: ReaderAst<Query__RepositoryDetail__param> = [
  {
    kind: "Linked",
    isFallible: true,
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
    condition: null,
    isUpdatable: false,
    refetchQueryIndex: null,
    selections: [
      {
        kind: "Resolver",
        alias: "IsStarred",
        arguments: null,
        readerArtifact: Repository__IsStarred__resolver_reader,
        usedRefetchQueries: [],
      },
      {
        kind: "Scalar",
        isFallible: false,
        fieldName: "nameWithOwner",
        alias: null,
        arguments: null,
        isUpdatable: false,
      },
      {
        kind: "Linked",
        isFallible: true,
        fieldName: "parent",
        alias: null,
        arguments: null,
        condition: null,
        isUpdatable: false,
        refetchQueryIndex: null,
        selections: [
          {
            kind: "Resolver",
            alias: "RepositoryLink",
            arguments: null,
            readerArtifact: Repository__RepositoryLink__resolver_reader,
            usedRefetchQueries: [],
          },
          {
            kind: "Scalar",
            isFallible: false,
            fieldName: "nameWithOwner",
            alias: null,
            arguments: null,
            isUpdatable: false,
          },
        ],
      },
      {
        kind: "Linked",
        isFallible: false,
        fieldName: "pullRequests",
        alias: null,
        arguments: [
          [
            "last",
            { kind: "Variable", name: "first" },
          ],
        ],
        condition: null,
        isUpdatable: false,
        refetchQueryIndex: null,
        selections: [
          {
            kind: "Resolver",
            alias: "PullRequestTable",
            arguments: null,
            readerArtifact: PullRequestConnection__PullRequestTable__resolver_reader,
            usedRefetchQueries: [],
          },
        ],
      },
    ],
  },
];

const artifact = (): ComponentReaderArtifact<
  Query__RepositoryDetail__param,
  ExtractSecondParam<typeof resolver>
> => ({
  kind: "ComponentReaderArtifact",
  fieldName: "RepositoryDetail",
  resolver,
  readerAst,
  hasUpdatable: false,
});

export default artifact;
