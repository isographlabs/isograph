import type {ComponentReaderArtifact, ExtractSecondParam, ReaderAst, RefetchQueryNormalizationArtifact} from '@isograph/react';
import { Query__HomePage__param } from './param_type';
import { HomePage as resolver } from '../../../HomeRoute.tsx';
import Query__Header from '../Header/resolver_reader';
import Query__HomePageList from '../HomePageList/resolver_reader';

const readerAst: ReaderAst<Query__HomePage__param> = [
  {
    kind: "Resolver",
    alias: "Header",
    arguments: null,
    readerArtifact: Query__Header,
    usedRefetchQueries: [],
  },
  {
    kind: "Resolver",
    alias: "HomePageList",
    arguments: null,
    readerArtifact: Query__HomePageList,
    usedRefetchQueries: [0, ],
  },
];

const artifact: ComponentReaderArtifact<
  Query__HomePage__param,
  ExtractSecondParam<typeof resolver>
> = {
  kind: "ComponentReaderArtifact",
  componentName: "Query.HomePage",
  resolver,
  readerAst,
};

export default artifact;
