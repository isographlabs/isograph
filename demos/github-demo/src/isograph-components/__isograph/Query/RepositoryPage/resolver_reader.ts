import type {ComponentReaderArtifact, ExtractSecondParam, ReaderAst } from '@isograph/react';
import { Query__RepositoryPage__param } from './param_type';
import { RepositoryPage as resolver } from '../../../RepositoryRoute';
import Query__Header__resolver_reader from '../../Query/Header/resolver_reader';
import Query__RepositoryDetail__resolver_reader from '../../Query/RepositoryDetail/resolver_reader';

const readerAst: ReaderAst<Query__RepositoryPage__param> = [
  {
    kind: "Resolver",
    fieldName: "Header",
    alias: "null",
    arguments: null,
    readerArtifact: Query__Header__resolver_reader,
    usedRefetchQueries: [],
  },
  {
    kind: "Resolver",
    fieldName: "RepositoryDetail",
    alias: "null",
    arguments: [
      [
        "repositoryName",
        { kind: "Variable", name: "repositoryName" },
      ],

      [
        "repositoryOwner",
        { kind: "Variable", name: "repositoryOwner" },
      ],

      [
        "first",
        { kind: "Variable", name: "first" },
      ],
    ],
    readerArtifact: Query__RepositoryDetail__resolver_reader,
    usedRefetchQueries: [],
  },
];

const artifact: ComponentReaderArtifact<
  Query__RepositoryPage__param,
  ExtractSecondParam<typeof resolver>
> = {
  kind: "ComponentReaderArtifact",
  fieldName: "RepositoryPage",
  resolver,
  readerAst,
  hasUpdatable: false,
};

export default artifact;
