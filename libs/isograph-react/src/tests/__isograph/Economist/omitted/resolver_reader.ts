import type { EagerReaderArtifact, ReaderAst } from '@isograph/react';
import { Economist__omitted__param } from './param_type';
import { Economist__omitted__output_type } from './output_type';
import { omitted as resolver } from '../../../startUpdate.test';

const readerAst: ReaderAst<Economist__omitted__param> = [
  {
    kind: "Scalar",
    fieldName: "name",
    alias: null,
    arguments: null,
    isUpdatable: false,
  },
];

const artifact: EagerReaderArtifact<
  Economist__omitted__param,
  Economist__omitted__output_type
> = {
  kind: "EagerReaderArtifact",
  fieldName: "Economist.omitted",
  resolver,
  readerAst,
  hasUpdatable: false,
};

export default artifact;
