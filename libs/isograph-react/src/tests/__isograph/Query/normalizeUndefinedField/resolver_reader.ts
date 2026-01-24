import type { EagerReaderArtifact, ReaderAst } from '@isograph/react';
import { Query__normalizeUndefinedField__param } from './param_type';
import { Query__normalizeUndefinedField__output_type } from './output_type';
import { normalizeUndefinedField as resolver } from '../../../normalizeData.test';

const readerAst: ReaderAst<Query__normalizeUndefinedField__param> = [
  {
    kind: "Linked",
    isFallible: false,
    fieldName: "me",
    alias: null,
    arguments: null,
    condition: null,
    isUpdatable: false,
    refetchQueryIndex: null,
    selections: [
      {
        kind: "Scalar",
        isFallible: false,
        fieldName: "name",
        alias: null,
        arguments: null,
        isUpdatable: false,
      },
    ],
  },
];

const artifact = (): EagerReaderArtifact<
  Query__normalizeUndefinedField__param,
  Query__normalizeUndefinedField__output_type
> => ({
  kind: "EagerReaderArtifact",
  fieldName: "normalizeUndefinedField",
  resolver,
  readerAst,
  hasUpdatable: false,
});

export default artifact;
