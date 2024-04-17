import type {ComponentReaderArtifact, ExtractSecondParam, ReaderAst, RefetchQueryNormalizationArtifact} from '@isograph/react';
import { Query__Header__param } from './param_type';
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

const artifact: ComponentReaderArtifact<
  Query__Header__param,
  ExtractSecondParam<typeof resolver>
> = {
  kind: "ComponentReaderArtifact",
  componentName: "Query.Header",
  resolver,
  readerAst,
};

export default artifact;
