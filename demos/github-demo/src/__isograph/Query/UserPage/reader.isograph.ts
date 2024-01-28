import type {ReaderArtifact, ReaderAst} from '@isograph/react';
import { UserPage as resolver } from '../../../isograph-components/UserRoute.tsx';
import Query__Header, { ReadOutType as Query__Header__outputType } from '../Header/reader.isograph';
import Query__UserDetail, { ReadOutType as Query__UserDetail__outputType } from '../UserDetail/reader.isograph';

// the type, when read out (either via useLazyReference or via graph)
export type ReadOutType = (React.FC<any>);

export type ReadFromStoreType = ResolverParameterType;

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

export type ResolverParameterType = { data:
{
  Header: Query__Header__outputType,
  UserDetail: Query__UserDetail__outputType,
},
[index: string]: any };

// The type, when returned from the resolver
export type ResolverReturnType = ReturnType<typeof resolver>;

const artifact: ReaderArtifact<ReadFromStoreType, ResolverParameterType, ReadOutType> = {
  kind: "ReaderArtifact",
  resolver: resolver as any,
  readerAst,
  variant: { kind: "Component", componentName: "Query.UserPage" },
};

export default artifact;
