import type {ComponentReaderArtifact, ExtractSecondParam, ReaderAst, RefetchQueryNormalizationArtifact} from '@isograph/react';
import { Query__HomePageList__param } from './param_type';
import { HomePageList as resolver } from '../../../HomePageList.tsx';
import User__RepositoryList__resolver_reader from '../../User/RepositoryList/resolver_reader';
import User____refetch__refetch_reader from '../../User/__refetch/refetch_reader';

const readerAst: ReaderAst<Query__HomePageList__param> = [
  {
    kind: "Linked",
    fieldName: "viewer",
    alias: null,
    arguments: null,
    selections: [
      {
        kind: "Scalar",
        fieldName: "login",
        alias: null,
        arguments: null,
      },
      {
        kind: "Scalar",
        fieldName: "name",
        alias: null,
        arguments: null,
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
        readerArtifact: User____refetch__refetch_reader,
        refetchQuery: 0,
      },
    ],
  },
];

const artifact: ComponentReaderArtifact<
  Query__HomePageList__param,
  ExtractSecondParam<typeof resolver>
> = {
  kind: "ComponentReaderArtifact",
  componentName: "Query.HomePageList",
  resolver,
  readerAst,
};

export default artifact;
