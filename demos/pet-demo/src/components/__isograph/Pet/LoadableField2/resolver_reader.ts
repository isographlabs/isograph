import type {ComponentReaderArtifact, ExtractSecondParam, ReaderAst, RefetchQueryNormalizationArtifact} from '@isograph/react';
import { Pet__LoadableField2__param } from './param_type';
import { Foo2 as resolver } from '../../../LoadableDemo';

const readerAst: ReaderAst<Pet__LoadableField2__param> = [
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
  Pet__LoadableField2__param,
  ExtractSecondParam<typeof resolver>
> = {
  kind: "ComponentReaderArtifact",
  componentName: "Pet.LoadableField2",
  resolver,
  readerAst,
};

export default artifact;
