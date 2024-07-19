import type {ComponentReaderArtifact, ExtractSecondParam, ReaderAst } from '@isograph/react';
import { Pet__LoadableField__param } from './param_type';
import { Foo as resolver } from '../../../LoadableDemo';

const readerAst: ReaderAst<Pet__LoadableField__param> = [
  {
    kind: "Scalar",
    fieldName: "tagline",
    alias: null,
    arguments: null,
  },
  {
    kind: "Scalar",
    fieldName: "alt_tagline",
    alias: null,
    arguments: null,
  },
];

const artifact: ComponentReaderArtifact<
  Pet__LoadableField__param,
  ExtractSecondParam<typeof resolver>
> = {
  kind: "ComponentReaderArtifact",
  componentName: "Pet.LoadableField",
  resolver,
  readerAst,
};

export default artifact;
