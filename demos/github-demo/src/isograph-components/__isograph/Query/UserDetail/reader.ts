import type {ReaderArtifact, ReaderAst} from '@isograph/react';
import { Query__UserDetail__param } from './param_type.ts';
import { Query__UserDetail__outputType } from './output_type.ts';
import { UserDetail as resolver } from '../../../UserDetail.tsx';
import User__RepositoryList from '../../User/RepositoryList/reader';

const readerAst: ReaderAst<Query__UserDetail__param> = [
  {
    kind: "Linked",
    fieldName: "user",
    alias: null,
    arguments: [
      [
        "login",
        { kind: "Variable", name: "userLogin" },
      ],
    ],
    selections: [
      {
        kind: "Scalar",
        fieldName: "name",
        alias: null,
        arguments: null,
      },
      {
        kind: "Resolver",
        alias: "RepositoryList",
        arguments: null,
        readerArtifact: User__RepositoryList,
        usedRefetchQueries: [],
      },
    ],
  },
];

const artifact: ReaderArtifact<
  Query__UserDetail__param,
  Query__UserDetail__outputType
> = {
  kind: "ReaderArtifact",
  fieldName: "UserDetail",
  resolver: resolver as any,
  readerAst,
  variant: { kind: "Component", componentName: "Query.UserDetail" },
};

export default artifact;
