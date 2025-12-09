import type { EagerReaderArtifact, ReaderAst } from '@isograph/react';
import { Query__nicknameErrors__param } from './param_type';
import { Query__nicknameErrors__output_type } from './output_type';
import { nicknameErrors as resolver } from '../../../normalizeData.test';
import Node__asEconomist__resolver_reader from '../../Node/asEconomist/resolver_reader';

const readerAst: ReaderAst<Query__nicknameErrors__param> = [
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
        kind: "Linked",
        fieldName: "asEconomist",
        alias: null,
        arguments: null,
        condition: Node__asEconomist__resolver_reader,
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
          {
            kind: "Scalar",
            fieldName: "nickname",
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
  Query__nicknameErrors__param,
  Query__nicknameErrors__output_type
> = {
  kind: "EagerReaderArtifact",
  fieldName: "nicknameErrors",
  resolver,
  readerAst,
  hasUpdatable: false,
};

export default artifact;
