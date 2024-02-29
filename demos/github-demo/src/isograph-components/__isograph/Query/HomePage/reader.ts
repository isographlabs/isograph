import type {ReaderArtifact, ReaderAst, ExtractSecondParam} from '@isograph/react';
import { HomePage as resolver } from '../../../HomeRoute.tsx';
import Query__Header, { Query__Header__outputType} from '../Header/reader';
import Query__HomePageList, { Query__HomePageList__outputType} from '../HomePageList/reader';

// the type, when read out (either via useLazyReference or via graph)
export type Query__HomePage__outputType = (React.FC<ExtractSecondParam<typeof resolver>>);

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

export type Query__HomePage__param = {
  Header: Query__Header__outputType,
  HomePageList: Query__HomePageList__outputType,
};

const artifact: ReaderArtifact<
  Query__HomePage__param,
  Query__HomePage__param,
  Query__HomePage__outputType
> = {
  kind: "ReaderArtifact",
  resolver: resolver as any,
  readerAst,
  variant: { kind: "Component", componentName: "Query.HomePage" },
};

export default artifact;
