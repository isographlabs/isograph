import type { EagerReaderArtifact, ReaderAst } from '@isograph/react';
import { Query__nodeField__param } from './param_type';
import { Query__nodeField__output_type } from './output_type';
import { nodeField as resolver } from '../../../nodeQuery';

const readerAst: ReaderAst<Query__nodeField__param> = [
  {
    kind: "Linked",
    isFallible: true,
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
        isFallible: false,
        fieldName: "id",
        alias: null,
        arguments: null,
        isUpdatable: false,
      },
    ],
  },
];

const artifact = (): EagerReaderArtifact<
  Query__nodeField__param,
  Query__nodeField__output_type
> => ({
  kind: "EagerReaderArtifact",
  fieldName: "nodeField",
  resolver,
  readerAst,
  hasUpdatable: false,
});

export default artifact;
