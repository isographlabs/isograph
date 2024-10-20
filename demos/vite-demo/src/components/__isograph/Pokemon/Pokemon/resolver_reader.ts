import type {ComponentReaderArtifact, ExtractSecondParam, ReaderAst } from '@isograph/react';
import { Pokemon__Pokemon__param } from './param_type';
import { Pokemon as resolver } from '../../../Pokemon';

const readerAst: ReaderAst<Pokemon__Pokemon__param> = [
  {
    kind: "Scalar",
    fieldName: "num",
    alias: null,
    arguments: null,
  },
  {
    kind: "Scalar",
    fieldName: "species",
    alias: null,
    arguments: null,
  },
  {
    kind: "Scalar",
    fieldName: "sprite",
    alias: null,
    arguments: null,
  },
  {
    kind: "Scalar",
    fieldName: "bulbapediaPage",
    alias: null,
    arguments: null,
  },
];

const artifact: ComponentReaderArtifact<
  Pokemon__Pokemon__param,
  ExtractSecondParam<typeof resolver>
> = {
  kind: "ComponentReaderArtifact",
  componentName: "Pokemon.Pokemon",
  resolver,
  readerAst,
};

export default artifact;
