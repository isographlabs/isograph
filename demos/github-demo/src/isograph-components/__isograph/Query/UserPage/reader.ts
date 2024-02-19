import type {ReaderArtifact, ReaderAst} from '@isograph/react';
import { UserPage as resolver } from '../../../UserRoute.tsx';
import Query__Header, { ReadOutType as Query__Header__outputType } from '../Header/reader';
import Query__UserDetail, { ReadOutType as Query__UserDetail__outputType } from '../UserDetail/reader';

// the type, when read out (either via useLazyReference or via graph)
export type ReadOutType = (React.FC<any>);

export type ReadFromStoreType = Query__UserPage__param;

const readerAst: ReaderAst<ReadFromStoreType> = [
  {
    kind: "Resolver",
    alias: "Header",
    arguments: null,
    readerArtifact: Query__Header,
    usedRefetchQueries: [],
  },
  {
    kind: "Resolver",
    alias: "UserDetail",
    arguments: null,
    readerArtifact: Query__UserDetail,
    usedRefetchQueries: [],
  },
];

export type Query__UserPage__param = { data:
{
  Header: Query__Header__outputType,
  UserDetail: Query__UserDetail__outputType,
},
[index: string]: any };

const artifact: ReaderArtifact<ReadFromStoreType, Query__UserPage__param, ReadOutType> = {
  kind: "ReaderArtifact",
  resolver: resolver as any,
  readerAst,
  variant: { kind: "Component", componentName: "Query.UserPage" },
};

export default artifact;
