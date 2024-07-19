import type { EagerReaderArtifact, ReaderAst } from '@isograph/react';
import { Query__nodeField__param } from './param_type';
import { Query__nodeField__output_type } from './output_type';
import { nodeField as resolver } from '../../../nodeQuery';

const readerAst: ReaderAst<Query__nodeField__param> = [
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
    selections: [
      {
        kind: "Scalar",
        fieldName: "id",
        alias: null,
        arguments: null,
      },
    ],
  },
];

const artifact: EagerReaderArtifact<
  Query__nodeField__param,
  Query__nodeField__output_type
> = {
  kind: "EagerReaderArtifact",
  resolver,
  readerAst,
};

export default artifact;
