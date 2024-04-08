import type {ReaderArtifact, ReaderAst} from '@isograph/react';
import { Query__UserPage__param } from './param_type.ts';
import { Query__UserPage__outputType } from './output_type.ts';
import { UserPage as resolver } from '../../../UserRoute.tsx';
import Query__Header from '../Header/reader';
import Query__UserDetail from '../UserDetail/reader';

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

const artifact: ReaderArtifact<
  Query__UserPage__param,
  Query__UserPage__outputType
> = {
  kind: "ReaderArtifact",
  resolver: resolver as any,
  readerAst,
  variant: { kind: "Component", componentName: "Query.UserPage" },
};

export default artifact;
