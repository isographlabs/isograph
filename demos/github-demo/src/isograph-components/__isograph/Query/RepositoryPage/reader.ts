import type {ReaderArtifact, ReaderAst} from '@isograph/react';
import { RepositoryPage as resolver } from '../../../RepositoryRoute.tsx';
import Query__Header, { ReadOutType as Query__Header__outputType } from '../Header/reader';
import Query__RepositoryDetail, { ReadOutType as Query__RepositoryDetail__outputType } from '../RepositoryDetail/reader';

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
    alias: "RepositoryDetail",
    arguments: null,
    readerArtifact: Query__RepositoryDetail,
    usedRefetchQueries: [],
  },
];

export type ResolverParameterType = { data:
{
  Header: Query__Header__outputType,
  RepositoryDetail: Query__RepositoryDetail__outputType,
},
[index: string]: any };

// The type, when returned from the resolver
export type ResolverReturnType = ReturnType<typeof resolver>;

const artifact: ReaderArtifact<ReadFromStoreType, ResolverParameterType, ReadOutType> = {
  kind: "ReaderArtifact",
  resolver: resolver as any,
  readerAst,
  variant: { kind: "Component", componentName: "Query.RepositoryPage" },
};

export default artifact;
