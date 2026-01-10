import type { EagerReaderArtifact, ReaderAst } from '@isograph/react';
import { Economist__errorsClientFieldField__param } from './param_type';
import { Economist__errorsClientFieldField__output_type } from './output_type';
import { errorsClientFieldField as resolver } from '../../../normalizeData.test';

const readerAst: ReaderAst<Economist__errorsClientFieldField__param> = [
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
  Economist__errorsClientFieldField__param,
  Economist__errorsClientFieldField__output_type
> = {
  kind: "EagerReaderArtifact",
  fieldName: "errorsClientFieldField",
  resolver,
  readerAst,
  hasUpdatable: false,
};

export default artifact;
