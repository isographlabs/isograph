import type {ReaderArtifact, ReaderAst} from '@isograph/react';
import { Query__Header__param } from './param_type.ts';
import { Query__Header__outputType } from './output_type.ts';
import { Header as resolver } from '../../../header.tsx';
import User__Avatar from '../../User/Avatar/reader';

const readerAst: ReaderAst<Query__Header__param> = [
  {
    kind: "Linked",
    fieldName: "viewer",
    alias: null,
    arguments: null,
    selections: [
      {
        kind: "Scalar",
        fieldName: "name",
        alias: null,
        arguments: null,
      },
      {
        kind: "Resolver",
        alias: "Avatar",
        arguments: null,
        readerArtifact: User__Avatar,
        usedRefetchQueries: [],
      },
    ],
  },
];

const artifact: ReaderArtifact<
  Query__Header__param,
  Query__Header__outputType
> = {
  kind: "ReaderArtifact",
  fieldName: "Header",
  resolver: resolver as any,
  readerAst,
  variant: { kind: "Component", componentName: "Query.Header" },
};

export default artifact;
