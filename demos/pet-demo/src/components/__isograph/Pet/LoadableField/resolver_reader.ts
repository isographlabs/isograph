import type {EagerReaderArtifact, ReaderAst, RefetchQueryNormalizationArtifact} from '@isograph/react';
import { Pet__LoadableField__param } from './param_type';
import { Pet__LoadableField__output_type } from './output_type';
import { Foo as resolver } from '../../../Loadable';

const readerAst: ReaderAst<Pet__LoadableField__param> = [
  {
    kind: "Scalar",
    fieldName: "name",
    alias: null,
    arguments: null,
  },
  {
    kind: "Scalar",
    fieldName: "tagline",
    alias: null,
    arguments: null,
  },
];

const artifact: EagerReaderArtifact<
  Pet__LoadableField__param,
  Pet__LoadableField__output_type
> = {
  kind: "EagerReaderArtifact",
  resolver,
  readerAst,
};

export default artifact;
