import type {ReaderArtifact, ReaderAst} from '@isograph/react';
import { HomePageList as resolver } from '../../../HomePageList.tsx';
import User__RepositoryList, { User__RepositoryList__outputType} from '../../User/RepositoryList/reader';
import User____refetch, { User____refetch__outputType} from '../../User/__refetch/reader';

// the type, when read out (either via useLazyReference or via graph)
export type Query__HomePageList__outputType = (React.FC<any>);

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

export type Query__HomePageList__param = { data:
{
  viewer: {
    login: string,
    name: (string | null),
    RepositoryList: User__RepositoryList__outputType,
    __refetch: User____refetch__outputType,
  },
},
[index: string]: any };

const artifact: ReaderArtifact<
  Query__HomePageList__param,
  Query__HomePageList__param,
  Query__HomePageList__outputType
> = {
  kind: "ReaderArtifact",
  resolver: resolver as any,
  readerAst,
  variant: { kind: "Component", componentName: "Query.HomePageList" },
};

export default artifact;
