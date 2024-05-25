import type {EagerReaderArtifact, ReaderAst, RefetchQueryNormalizationArtifact} from '@isograph/react';
import { Query__nodeField__param } from './param_type';
import { Query__nodeField__outputType } from './output_type';
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

const artifact: EagerReaderArtifact<
  Query__nodeField__param,
  Query__nodeField__outputType
> = {
  kind: "EagerReaderArtifact",
  resolver,
  readerAst,
};

export default artifact;
