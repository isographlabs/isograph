import type {ComponentReaderArtifact, ExtractSecondParam, ReaderAst, RefetchQueryNormalizationArtifact} from '@isograph/react';
import { Query__HomePageList__param } from './param_type';
import { HomePageList as resolver } from '../../../HomePageList.tsx';
import User__RepositoryList from '../../User/RepositoryList/resolver_reader';
import User____refetch from '../../User/__refetch/resolver_reader';

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
        readerArtifact: User__RepositoryList,
        usedRefetchQueries: [],
      },
      {
        kind: "RefetchField",
        alias: "__refetch",
        readerArtifact: User____refetch,
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
