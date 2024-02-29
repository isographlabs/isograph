import type {ReaderArtifact, ReaderAst, ExtractSecondParam} from '@isograph/react';
import { RepositoryPage as resolver } from '../../../RepositoryRoute.tsx';
import Query__Header, { Query__Header__outputType} from '../Header/reader';
import Query__RepositoryDetail, { Query__RepositoryDetail__outputType} from '../RepositoryDetail/reader';

// the type, when read out (either via useLazyReference or via graph)
export type Query__RepositoryPage__outputType = (React.FC<ExtractSecondParam<typeof resolver>>);

const readerAst: ReaderAst<Query__RepositoryPage__param> = [
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

export type Query__RepositoryPage__param = {
  Header: Query__Header__outputType,
  RepositoryDetail: Query__RepositoryDetail__outputType,
};

const artifact: ReaderArtifact<
  Query__RepositoryPage__param,
  Query__RepositoryPage__outputType
> = {
  kind: "ReaderArtifact",
  resolver: resolver as any,
  readerAst,
  variant: { kind: "Component", componentName: "Query.RepositoryPage" },
};

export default artifact;
