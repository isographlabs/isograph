import type {EagerReaderArtifact, ReaderAst, RefetchQueryNormalizationArtifact} from '@isograph/react';
import { Pet__petSuperName__param } from './param_type';
import { Pet__petSuperName__outputType } from './output_type';
import { petSuperName as resolver } from '../../../HomeRoute.tsx';

const readerAst: ReaderAst<Pet__petSuperName__param> = [
  {
    kind: "Scalar",
    fieldName: "name",
    alias: null,
    arguments: null,
  },
];

const artifact: EagerReaderArtifact<
  Pet__petSuperName__param,
  Pet__petSuperName__outputType
> = {
  kind: "EagerReaderArtifact",
  resolver,
  readerAst,
};

export default artifact;
