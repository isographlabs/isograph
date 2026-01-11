import type { EagerReaderArtifact, ReaderAst } from '@isograph/react';
import { Query__errorsClientField__param } from './param_type';
import { Query__errorsClientField__output_type } from './output_type';
import { errorsClientField as resolver } from '../../../normalizeData.test';
import Economist__errorsClientFieldField__resolver_reader from '../../Economist/errorsClientFieldField/resolver_reader';
import Node__asEconomist__resolver_reader from '../../Node/asEconomist/resolver_reader';

const readerAst: ReaderAst<Query__errorsClientField__param> = [
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
            kind: "Resolver",
            alias: "errorsClientFieldField",
            arguments: null,
            readerArtifact: Economist__errorsClientFieldField__resolver_reader,
            usedRefetchQueries: [],
          },
        ],
      },
    ],
  },
];

const artifact: EagerReaderArtifact<
  Query__errorsClientField__param,
  Query__errorsClientField__output_type
> = {
  kind: "EagerReaderArtifact",
  fieldName: "errorsClientField",
  resolver,
  readerAst,
  hasUpdatable: false,
};

export default artifact;
