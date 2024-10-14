import type {ComponentReaderArtifact, ExtractSecondParam, ReaderAst } from '@isograph/react';
import { Query__UserPage__param } from './param_type';
import { UserPage as resolver } from '../../../UserRoute';
import Query__Header__resolver_reader from '../../Query/Header/resolver_reader';
import Query__UserDetail__resolver_reader from '../../Query/UserDetail/resolver_reader';

const readerAst: ReaderAst<Query__UserPage__param> = [
  {
    kind: "Resolver",
    alias: "Header",
    arguments: null,
    readerArtifact: Query__Header__resolver_reader,
    usedRefetchQueries: [],
  },
  {
    kind: "Resolver",
    alias: "UserDetail",
    arguments: [
      [
        "userLogin",
        { kind: "Variable", name: "userLogin" },
      ],
    ],
    readerArtifact: Query__UserDetail__resolver_reader,
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
