import type {EagerReaderArtifact, ReaderAst, RefetchQueryNormalizationArtifact} from '@isograph/react';
import { Pet__Foo__param } from './param_type';
import { Pet__Foo__output_type } from './output_type';
import { Foo as resolver } from '../../../Loadable';

const readerAst: ReaderAst<Pet__Foo__param> = [
  {
    kind: "Scalar",
    fieldName: "name",
    alias: null,
    arguments: null,
  },
];

const artifact: EagerReaderArtifact<
  Pet__Foo__param,
  Pet__Foo__output_type
> = {
  kind: "EagerReaderArtifact",
  resolver,
  readerAst,
};

export default artifact;
