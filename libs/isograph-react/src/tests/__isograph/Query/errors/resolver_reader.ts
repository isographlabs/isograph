import type { EagerReaderArtifact, ReaderAst } from '@isograph/react';
import { Query__errors__param } from './param_type';
import { Query__errors__output_type } from './output_type';
import { errors as resolver } from '../../../normalizeData.test';
import Node__asEconomist__resolver_reader from '../../Node/asEconomist/resolver_reader';

const readerAst: ReaderAst<Query__errors__param> = [
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
        kind: "Linked",
        isFallible: false,
        fieldName: "asEconomist",
        alias: null,
        arguments: null,
        condition: Node__asEconomist__resolver_reader,
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
    ],
  },
];

const artifact = (): EagerReaderArtifact<
  Query__errors__param,
  Query__errors__output_type
> => ({
  kind: "EagerReaderArtifact",
  fieldName: "errors",
  resolver,
  readerAst,
  hasUpdatable: false,
});

export default artifact;
