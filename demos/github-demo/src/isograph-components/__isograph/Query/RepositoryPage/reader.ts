import type {ComponentReaderArtifact, ExtractSecondParam, ReaderAst, RefetchQueryNormalizationArtifact} from '@isograph/react';
import { Query__RepositoryPage__param } from './param_type';
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

const artifact: ComponentReaderArtifact<
  Query__RepositoryPage__param,
  ExtractSecondParam<typeof resolver>
> = {
  kind: "ComponentReaderArtifact",
  componentName: "Query.RepositoryPage",
  resolver,
  readerAst,
};

export default artifact;
