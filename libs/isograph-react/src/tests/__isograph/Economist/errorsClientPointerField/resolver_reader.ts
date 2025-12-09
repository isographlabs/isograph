import type { EagerReaderArtifact, ReaderAst } from '@isograph/react';
import { Economist__errorsClientPointerField__param } from './param_type';
import { Economist__errorsClientPointerField__output_type } from './output_type';
import { errorsClientPointerField as resolver } from '../../../normalizeData.test';

const readerAst: ReaderAst<Economist__errorsClientPointerField__param> = [
  {
    kind: "Scalar",
    fieldName: "id",
    alias: null,
    arguments: null,
    isUpdatable: false,
  },
  {
    kind: "Scalar",
    fieldName: "nickname",
    alias: null,
    arguments: null,
    isUpdatable: false,
  },
];

const artifact: EagerReaderArtifact<
  Economist__errorsClientPointerField__param,
  Economist__errorsClientPointerField__output_type
> = {
  kind: "EagerReaderArtifact",
  fieldName: "errorsClientPointerField",
  resolver,
  readerAst,
  hasUpdatable: false,
};

export default artifact;
