import type {ComponentReaderArtifact, ExtractSecondParam, ReaderAst } from '@isograph/react';
import { Starrable__IsStarred__param } from './param_type';
import { IsStarred as resolver } from '../../../RepositoryDetail';

const readerAst: ReaderAst<Starrable__IsStarred__param> = [
  {
    kind: "Scalar",
    fieldName: "stargazerCount",
    alias: null,
    arguments: null,
  },
  {
    kind: "Scalar",
    fieldName: "viewerHasStarred",
    alias: null,
    arguments: null,
  },
];

const artifact: ComponentReaderArtifact<
  Starrable__IsStarred__param,
  ExtractSecondParam<typeof resolver>
> = {
  kind: "ComponentReaderArtifact",
  fieldName: "Starrable.IsStarred",
  resolver,
  readerAst,
  hasUpdatable: false,
};

export default artifact;
