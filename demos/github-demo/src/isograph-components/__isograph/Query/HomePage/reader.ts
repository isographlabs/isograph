import type {ReaderArtifact, ReaderAst} from '@isograph/react';
import { Query__HomePage__param } from './param_type.ts';
import { Query__HomePage__outputType } from './output_type.ts';
import { HomePage as resolver } from '../../../HomeRoute.tsx';
import Query__Header from '../Header/reader';
import Query__HomePageList from '../HomePageList/reader';

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

const artifact: ReaderArtifact<
  Query__HomePage__param,
  Query__HomePage__outputType
> = {
  kind: "ReaderArtifact",
  fieldName: "HomePage",
  resolver: resolver as any,
  readerAst,
  variant: { kind: "Component", componentName: "Query.HomePage" },
};

export default artifact;
