import type {ComponentReaderArtifact, ExtractSecondParam, ReaderAst } from '@isograph/react';
import { Repository__IsStarred__param } from './param_type';
import { IsStarred as resolver } from '../../../RepositoryDetail';

const readerAst: ReaderAst<Repository__IsStarred__param> = [
  {
    kind: "Scalar",
    isFallible: false,
    fieldName: "stargazerCount",
    alias: null,
    arguments: null,
    isUpdatable: false,
  },
  {
    kind: "Scalar",
    isFallible: false,
    fieldName: "viewerHasStarred",
    alias: null,
    arguments: null,
    isUpdatable: false,
  },
];

const artifact = (): ComponentReaderArtifact<
  Repository__IsStarred__param,
  ExtractSecondParam<typeof resolver>
> => ({
  kind: "ComponentReaderArtifact",
  fieldName: "IsStarred",
  resolver,
  readerAst,
  hasUpdatable: false,
});

export default artifact;
