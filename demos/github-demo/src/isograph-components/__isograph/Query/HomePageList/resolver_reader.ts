import type {ComponentReaderArtifact, ExtractSecondParam, ReaderAst } from '@isograph/react';
import { Query__HomePageList__param } from './param_type';
import { HomePageList as resolver } from '../../../HomePageList';
import User__RepositoryList__resolver_reader from '../../User/RepositoryList/resolver_reader';
import User____refetch__refetch_reader from '../../User/__refetch/refetch_reader';

const readerAst: ReaderAst<Query__HomePageList__param> = [
  {
    kind: "Linked",
    fieldName: "viewer",
    alias: null,
    arguments: null,
    condition: null,
    isUpdatable: false,
    refetchQueryIndex: null,
    selections: [
      {
        kind: "Scalar",
        fieldName: "login",
        alias: null,
        arguments: null,
        isUpdatable: false,
      },
      {
        kind: "Scalar",
        fieldName: "name",
        alias: null,
        arguments: null,
        isUpdatable: false,
      },
      {
        kind: "Resolver",
        alias: "RepositoryList",
        arguments: null,
        readerArtifact: User__RepositoryList__resolver_reader,
        usedRefetchQueries: [],
      },
      {
        kind: "ImperativelyLoadedField",
        alias: "__refetch",
        refetchReaderArtifact: User____refetch__refetch_reader,
        refetchQueryIndex: 0,
        name: "__refetch",
      },
    ],
  },
];

const artifact = (): ComponentReaderArtifact<
  Query__HomePageList__param,
  ExtractSecondParam<typeof resolver>
> => ({
  kind: "ComponentReaderArtifact",
  fieldName: "HomePageList",
  resolver,
  readerAst,
  hasUpdatable: false,
});

export default artifact;
