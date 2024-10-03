import type {ComponentReaderArtifact, ExtractSecondParam, ReaderAst } from '@isograph/react';
import { Query__RepositoryPage__param } from './param_type';
import { RepositoryPage as resolver } from '../../../RepositoryRoute';
import Query__Header__resolver_reader from '../../Query/Header/resolver_reader';
import Query__RepositoryDetail__resolver_reader from '../../Query/RepositoryDetail/resolver_reader';

const readerAst: ReaderAst<Query__RepositoryPage__param> = [
  {
    kind: "Resolver",
    alias: "Header",
    arguments: null,
    readerArtifact: Query__Header__resolver_reader,
    usedRefetchQueries: [],
  },
  {
    kind: "Resolver",
    alias: "RepositoryDetail",
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
  componentName: "Query.RepositoryPage",
  resolver,
  readerAst,
};

export default artifact;
