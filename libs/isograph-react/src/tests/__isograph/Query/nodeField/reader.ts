import type {ReaderArtifact, ReaderAst} from '@isograph/react';
import { Query__nodeField__param } from './param_type.ts';
import { Query__nodeField__outputType } from './output_type.ts';
import { nodeField as resolver } from '../../../nodeQuery.ts';

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

const artifact: ReaderArtifact<
  Query__nodeField__param,
  Query__nodeField__outputType
> = {
  kind: "ReaderArtifact",
  fieldName: "nodeField",
  resolver: resolver as any,
  readerAst,
  variant: { kind: "Eager" },
};

export default artifact;
