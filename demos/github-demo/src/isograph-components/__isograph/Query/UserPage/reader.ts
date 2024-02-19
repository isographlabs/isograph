import type {ReaderArtifact, ReaderAst} from '@isograph/react';
import { UserPage as resolver } from '../../../UserRoute.tsx';
import Query__Header, { Query__Header__outputType} from '../Header/reader';
import Query__UserDetail, { Query__UserDetail__outputType} from '../UserDetail/reader';

// the type, when read out (either via useLazyReference or via graph)
export type Query__UserPage__outputType = (React.FC<any>);

const readerAst: ReaderAst<Query__UserPage__param> = [
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

const artifact: ReaderArtifact<
  Query__UserPage__param,
  Query__UserPage__param,
  Query__UserPage__outputType
> = {
  kind: "ReaderArtifact",
  resolver: resolver as any,
  readerAst,
  variant: { kind: "Component", componentName: "Query.UserPage" },
};

export default artifact;
