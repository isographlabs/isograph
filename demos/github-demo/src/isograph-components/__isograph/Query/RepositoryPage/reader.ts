import type {ReaderArtifact, ReaderAst} from '@isograph/react';
import { Query__RepositoryPage__param } from './param_type.ts';
import { Query__RepositoryPage__outputType } from './output_type.ts';
import { RepositoryPage as resolver } from '../../../RepositoryRoute.tsx';
import Query__Header from '../Header/reader';
import Query__RepositoryDetail from '../RepositoryDetail/reader';

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

const artifact: ReaderArtifact<
  Query__RepositoryPage__param,
  Query__RepositoryPage__outputType
> = {
  kind: "ReaderArtifact",
  fieldName: "RepositoryPage",
  resolver: resolver as any,
  readerAst,
  variant: { kind: "Component", componentName: "Query.RepositoryPage" },
};

export default artifact;
