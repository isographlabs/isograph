import type {ComponentReaderArtifact, ExtractSecondParam, ReaderAst } from '@isograph/react';
import { Query__Header__param } from './param_type';
import { Header as resolver } from '../../../header';
import User__Avatar__resolver_reader from '../../User/Avatar/resolver_reader';

const readerAst: ReaderAst<Query__Header__param> = [
  {
    kind: "Linked",
    isFallible: false,
    fieldName: "viewer",
    alias: null,
    arguments: null,
    condition: null,
    isUpdatable: false,
    refetchQueryIndex: null,
    selections: [
      {
        kind: "Scalar",
        isFallible: true,
        fieldName: "name",
        alias: null,
        arguments: null,
        isUpdatable: false,
      },
      {
        kind: "Resolver",
        alias: "Avatar",
        arguments: null,
        readerArtifact: User__Avatar__resolver_reader,
        usedRefetchQueries: [],
      },
    ],
  },
];

const artifact = (): ComponentReaderArtifact<
  Query__Header__param,
  ExtractSecondParam<typeof resolver>
> => ({
  kind: "ComponentReaderArtifact",
  fieldName: "Header",
  resolver,
  readerAst,
  hasUpdatable: false,
});

export default artifact;
