import type {ComponentReaderArtifact, ExtractSecondParam, ReaderAst } from '@isograph/react';
import { planets__HomePage__param } from './param_type';
import { HomePage as resolver } from '../../../HomePage';

const readerAst: ReaderAst<planets__HomePage__param> = [
  {
    kind: "Scalar",
    isFallible: false,
    fieldName: "name",
    alias: "planetName",
    arguments: null,
    isUpdatable: false,
  },
  {
    kind: "Scalar",
    isFallible: false,
    fieldName: "id",
    alias: null,
    arguments: null,
    isUpdatable: false,
  },
  {
    kind: "Scalar",
    isFallible: false,
    fieldName: "climate",
    alias: null,
    arguments: null,
    isUpdatable: false,
  },
  {
    kind: "Scalar",
    isFallible: false,
    fieldName: "surface_water",
    alias: null,
    arguments: null,
    isUpdatable: false,
  },
  {
    kind: "Scalar",
    isFallible: false,
    fieldName: "orbital_period",
    alias: null,
    arguments: null,
    isUpdatable: false,
  },
];

const artifact = (): ComponentReaderArtifact<
  planets__HomePage__param,
  ExtractSecondParam<typeof resolver>
> => ({
  kind: "ComponentReaderArtifact",
  fieldName: "HomePage",
  resolver,
  readerAst,
  hasUpdatable: false,
});

export default artifact;
