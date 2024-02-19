import type {ReaderArtifact, ReaderAst} from '@isograph/react';
import { PullRequest as resolver } from '../../../PullRequestRoute.tsx';
import Query__Header, { Query__Header__outputType} from '../Header/reader';
import Query__PullRequestDetail, { Query__PullRequestDetail__outputType} from '../PullRequestDetail/reader';

// the type, when read out (either via useLazyReference or via graph)
export type Query__PullRequest__outputType = (React.FC<any>);

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

export type Query__PullRequest__param = { data:
{
  Header: Query__Header__outputType,
  PullRequestDetail: Query__PullRequestDetail__outputType,
},
[index: string]: any };

const artifact: ReaderArtifact<
  Query__PullRequest__param,
  Query__PullRequest__param,
  Query__PullRequest__outputType
> = {
  kind: "ReaderArtifact",
  resolver: resolver as any,
  readerAst,
  variant: { kind: "Component", componentName: "Query.PullRequest" },
};

export default artifact;
