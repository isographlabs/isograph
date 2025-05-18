import type { EagerReaderArtifact, ReaderAst } from '@isograph/react';
import { Query__linkedUpdate__param } from './param_type';
import { Query__linkedUpdate__output_type } from './output_type';
import { linkedUpdate as resolver } from '../../../startUpdate.test';
import Economist__asEconomist__resolver_reader from '../../Economist/asEconomist/resolver_reader';

const readerAst: ReaderAst<Query__linkedUpdate__param> = [
  {
    kind: "Linked",
    fieldName: "node",
    alias: null,
    arguments: [
      [
        "id",
        { kind: "Literal", value: 0 },
      ],
    ],
    condition: null,
    isUpdatable: true,
    selections: [
      {
        kind: "Linked",
        fieldName: "asEconomist",
        alias: null,
        arguments: null,
        condition: Economist__asEconomist__resolver_reader,
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
      },
    ],
  },
  {
    kind: "Linked",
    fieldName: "node",
    alias: "john_stuart_mill",
    arguments: [
      [
        "id",
        { kind: "Literal", value: 1 },
      ],
    ],
    condition: null,
    isUpdatable: false,
    selections: [
      {
        kind: "Link",
        alias: "link",
      },
      {
        kind: "Linked",
        fieldName: "asEconomist",
        alias: null,
        arguments: null,
        condition: Economist__asEconomist__resolver_reader,
        isUpdatable: false,
        selections: [
          {
            kind: "Scalar",
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

const artifact: EagerReaderArtifact<
  Query__linkedUpdate__param,
  Query__linkedUpdate__output_type
> = {
  kind: "EagerReaderArtifact",
  fieldName: "Query.linkedUpdate",
  resolver,
  readerAst,
  hasUpdatable: true,
};

export default artifact;
