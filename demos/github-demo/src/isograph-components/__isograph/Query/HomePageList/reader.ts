import type {ReaderArtifact, ReaderAst} from '@isograph/react';
import { Query__HomePageList__param } from './param_type.ts';
import { Query__HomePageList__outputType } from './output_type.ts';
import { HomePageList as resolver } from '../../../HomePageList.tsx';
import User__RepositoryList from '../../User/RepositoryList/reader';
import User____refetch from '../../User/__refetch/reader';

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

const artifact: ReaderArtifact<
  Query__HomePageList__param,
  Query__HomePageList__outputType
> = {
  kind: "ReaderArtifact",
  resolver: resolver as any,
  readerAst,
  variant: { kind: "Component", componentName: "Query.HomePageList" },
};

export default artifact;
