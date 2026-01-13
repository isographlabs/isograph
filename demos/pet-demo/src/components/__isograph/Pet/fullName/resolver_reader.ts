import type { EagerReaderArtifact, ReaderAst } from '@isograph/react';
import { Pet__fullName__param } from './param_type';
import { Pet__fullName__output_type } from './output_type';
import { fullName as resolver } from '../../../Pet/fullName';

const readerAst: ReaderAst<Pet__fullName__param> = [
  {
    kind: "Scalar",
    fieldName: "firstName",
    alias: null,
    arguments: null,
    isUpdatable: false,
  },
  {
    kind: "Scalar",
    fieldName: "lastName",
    alias: null,
    arguments: null,
    isUpdatable: false,
  },
];

const artifact = (): EagerReaderArtifact<
  Pet__fullName__param,
  Pet__fullName__output_type
> => ({
  kind: "EagerReaderArtifact",
  fieldName: "fullName",
  resolver,
  readerAst,
  hasUpdatable: false,
});

export default artifact;
