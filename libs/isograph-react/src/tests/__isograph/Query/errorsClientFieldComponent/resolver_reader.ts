import type { EagerReaderArtifact, ReaderAst } from '@isograph/react';
import { Query__errorsClientFieldComponent__param } from './param_type';
import { Query__errorsClientFieldComponent__output_type } from './output_type';
import { errorsClientFieldComponent as resolver } from '../../../normalizeData.test';
import Economist__errorsClientFieldComponentField__resolver_reader from '../../Economist/errorsClientFieldComponentField/resolver_reader';
import Node__asEconomist__resolver_reader from '../../Node/asEconomist/resolver_reader';

const readerAst: ReaderAst<Query__errorsClientFieldComponent__param> = [
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
            kind: "Resolver",
            alias: "errorsClientFieldComponentField",
            arguments: null,
            readerArtifact: Economist__errorsClientFieldComponentField__resolver_reader,
            usedRefetchQueries: [],
          },
        ],
      },
    ],
  },
];

const artifact = (): EagerReaderArtifact<
  Query__errorsClientFieldComponent__param,
  Query__errorsClientFieldComponent__output_type
> => ({
  kind: "EagerReaderArtifact",
  fieldName: "errorsClientFieldComponent",
  resolver,
  readerAst,
  hasUpdatable: false,
});

export default artifact;
