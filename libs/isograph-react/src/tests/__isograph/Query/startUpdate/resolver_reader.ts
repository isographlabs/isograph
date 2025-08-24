import type { EagerReaderArtifact, ReaderAst } from '@isograph/react';
import { Query__startUpdate__param } from './param_type';
import { Query__startUpdate__output_type } from './output_type';
import { startUpdate as resolver } from '../../../startUpdate.test';
import Node__asEconomist__resolver_reader from '../../Node/asEconomist/resolver_reader';

const readerAst: ReaderAst<Query__startUpdate__param> = [
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
    selections: [
      {
        kind: "Linked",
        fieldName: "asEconomist",
        alias: null,
        arguments: null,
        condition: Node__asEconomist__resolver_reader,
        isUpdatable: false,
        selections: [
          {
            kind: "Scalar",
            fieldName: "name",
            alias: null,
            arguments: null,
            isUpdatable: true,
          },
        ],
        refetchQueryIndex: null,
      },
    ],
    refetchQueryIndex: null,
  },
];

const artifact: EagerReaderArtifact<
  Query__startUpdate__param,
  Query__startUpdate__output_type
> = {
  kind: "EagerReaderArtifact",
  fieldName: "Query.startUpdate",
  resolver,
  readerAst,
  hasUpdatable: true,
};

export default artifact;
