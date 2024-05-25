import type {ComponentReaderArtifact, ExtractSecondParam, ReaderAst, RefetchQueryNormalizationArtifact} from '@isograph/react';
import { Query__UserPage__param } from './param_type';
import { UserPage as resolver } from '../../../UserRoute.tsx';
import Query__Header from '../Header/resolver_reader';
import Query__UserDetail from '../UserDetail/resolver_reader';

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

const artifact: ComponentReaderArtifact<
  Query__UserPage__param,
  ExtractSecondParam<typeof resolver>
> = {
  kind: "ComponentReaderArtifact",
  componentName: "Query.UserPage",
  resolver,
  readerAst,
};

export default artifact;
