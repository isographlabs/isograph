import type {ComponentReaderArtifact, ExtractSecondParam, ReaderAst } from '@isograph/react';
import { Economist__errorsClientFieldComponentField__param } from './param_type';
import { errorsClientFieldComponentField as resolver } from '../../../normalizeData.test';

const readerAst: ReaderAst<Economist__errorsClientFieldComponentField__param> = [
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

const artifact: ComponentReaderArtifact<
  Economist__errorsClientFieldComponentField__param,
  ExtractSecondParam<typeof resolver>
> = {
  kind: "ComponentReaderArtifact",
  fieldName: "errorsClientFieldComponentField",
  resolver,
  readerAst,
  hasUpdatable: false,
};

export default artifact;
