import type { EagerReaderArtifact, ReaderAst } from '@isograph/react';
import { Query__errorsSecond__param } from './param_type';
import { Query__errorsSecond__output_type } from './output_type';
import { errorsSecond as resolver } from '../../../normalizeData.test';

const readerAst: ReaderAst<Query__errorsSecond__param> = [
  {
    kind: "Linked",
    fieldName: "node",
    alias: null,
    arguments: [
      [
        "id",
        { kind: "Variable", name: "id" },
      ],
    ],
    condition: null,
    isUpdatable: false,
    refetchQueryIndex: null,
    selections: [
      {
        kind: "Scalar",
        fieldName: "id",
        alias: null,
        arguments: null,
        isUpdatable: false,
      },
    ],
  },
];

const artifact: EagerReaderArtifact<
  Query__errorsSecond__param,
  Query__errorsSecond__output_type
> = {
  kind: "EagerReaderArtifact",
  fieldName: "Query.errorsSecond",
  resolver,
  readerAst,
  hasUpdatable: false,
};

export default artifact;
