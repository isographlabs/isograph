import type {ComponentReaderArtifact, ExtractSecondParam, ReaderAst } from '@isograph/react';
import { Repository__RepositoryRow__param } from './param_type';
import { RepositoryRow as resolver } from '../../../UserRepositoryList';
import Repository__RepositoryLink__resolver_reader from '../../Repository/RepositoryLink/resolver_reader';

const readerAst: ReaderAst<Repository__RepositoryRow__param> = [
  {
    kind: "Resolver",
    alias: "RepositoryLink",
    arguments: null,
    readerArtifact: Repository__RepositoryLink__resolver_reader,
    usedRefetchQueries: [],
  },
  {
    kind: "Scalar",
    fieldName: "name",
    alias: null,
    arguments: null,
    isUpdatable: false,
  },
  {
    kind: "Scalar",
    fieldName: "nameWithOwner",
    alias: null,
    arguments: null,
    isUpdatable: false,
  },
  {
    kind: "Scalar",
    fieldName: "description",
    alias: null,
    arguments: null,
    isUpdatable: false,
  },
  {
    kind: "Scalar",
    fieldName: "forkCount",
    alias: null,
    arguments: null,
    isUpdatable: false,
  },
  {
    kind: "Linked",
    fieldName: "pullRequests",
    alias: null,
    arguments: null,
    condition: null,
    isUpdatable: false,
    refetchQueryIndex: null,
    selections: [
      {
        kind: "Scalar",
        fieldName: "totalCount",
        alias: null,
        arguments: null,
        isUpdatable: false,
      },
    ],
  },
  {
    kind: "Scalar",
    fieldName: "stargazerCount",
    alias: null,
    arguments: null,
    isUpdatable: false,
  },
  {
    kind: "Linked",
    fieldName: "watchers",
    alias: null,
    arguments: null,
    condition: null,
    isUpdatable: false,
    refetchQueryIndex: null,
    selections: [
      {
        kind: "Scalar",
        fieldName: "totalCount",
        alias: null,
        arguments: null,
        isUpdatable: false,
      },
    ],
  },
];

const artifact = (): ComponentReaderArtifact<
  Repository__RepositoryRow__param,
  ExtractSecondParam<typeof resolver>
> => ({
  kind: "ComponentReaderArtifact",
  fieldName: "RepositoryRow",
  resolver,
  readerAst,
  hasUpdatable: false,
});

export default artifact;
