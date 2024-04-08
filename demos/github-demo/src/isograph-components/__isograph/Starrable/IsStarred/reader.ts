import type {ReaderArtifact, ReaderAst} from '@isograph/react';
import { Starrable__IsStarred__param } from './param_type.ts';
import { Starrable__IsStarred__outputType } from './output_type.ts';
import { IsStarred as resolver } from '../../../RepositoryDetail.tsx';

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

const artifact: ReaderArtifact<
  Starrable__IsStarred__param,
  Starrable__IsStarred__outputType
> = {
  kind: "ReaderArtifact",
  resolver: resolver as any,
  readerAst,
  variant: { kind: "Component", componentName: "Starrable.IsStarred" },
};

export default artifact;
