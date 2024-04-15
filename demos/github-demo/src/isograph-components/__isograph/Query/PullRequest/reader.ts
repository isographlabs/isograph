import type {ReaderArtifact, ReaderAst} from '@isograph/react';
import { Query__PullRequest__param } from './param_type.ts';
import { Query__PullRequest__outputType } from './output_type.ts';
import { PullRequest as resolver } from '../../../PullRequestRoute.tsx';
import Query__Header from '../Header/reader';
import Query__PullRequestDetail from '../PullRequestDetail/reader';

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

const artifact: ReaderArtifact<
  Query__PullRequest__param,
  Query__PullRequest__outputType
> = {
  kind: "ReaderArtifact",
  resolver: resolver as any,
  readerAst,
  variant: { kind: "Component", componentName: "Query.PullRequest" },
};

export default artifact;
