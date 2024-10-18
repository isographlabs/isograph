import type { EagerReaderArtifact, ReaderAst } from '@isograph/react';
import { Query__subquery__param } from './param_type';
import { Query__subquery__output_type } from './output_type';
import { subquery as resolver } from '../../../normalizeData.test';

const readerAst: ReaderAst<Query__subquery__param> = [
  {
    kind: "Linked",
    fieldName: "query",
    alias: null,
    arguments: null,
    selections: [
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
    ],
  },
];

const artifact: EagerReaderArtifact<
  Query__subquery__param,
  Query__subquery__output_type
> = {
  kind: "EagerReaderArtifact",
  resolver,
  readerAst,
};

export default artifact;
