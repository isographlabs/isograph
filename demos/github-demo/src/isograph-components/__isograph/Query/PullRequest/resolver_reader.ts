import type {ComponentReaderArtifact, ExtractSecondParam, ReaderAst, RefetchQueryNormalizationArtifact} from '@isograph/react';
import { Query__PullRequest__param } from './param_type';
import { PullRequest as resolver } from '../../../PullRequestRoute.tsx';
import Query__Header from '../Header/resolver_reader';
import Query__PullRequestDetail from '../PullRequestDetail/resolver_reader';

const readerAst: ReaderAst<Query__PullRequest__param> = [
  {
    kind: "Resolver",
    alias: "Header",
    arguments: null,
    readerArtifact: Query__Header,
    usedRefetchQueries: [],
  },
  {
    kind: "Resolver",
    alias: "PullRequestDetail",
    arguments: null,
    readerArtifact: Query__PullRequestDetail,
    usedRefetchQueries: [],
  },
];

const artifact: ComponentReaderArtifact<
  Query__PullRequest__param,
  ExtractSecondParam<typeof resolver>
> = {
  kind: "ComponentReaderArtifact",
  componentName: "Query.PullRequest",
  resolver,
  readerAst,
};

export default artifact;
