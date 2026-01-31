import type { EagerReaderArtifact, ReaderAst } from '@isograph/react';
import { Query__errorsClientPointer__param } from './param_type';
import { Query__errorsClientPointer__output_type } from './output_type';
import { errorsClientPointer as resolver } from '../../../normalizeData.test';
import Economist__errorsClientPointerField__resolver_reader from '../../Economist/errorsClientPointerField/resolver_reader';
import Node__asEconomist__resolver_reader from '../../Node/asEconomist/resolver_reader';

const readerAst: ReaderAst<Query__errorsClientPointer__param> = [
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
            kind: "Linked",
            isFallible: false,
            fieldName: "errorsClientPointerField",
            alias: null,
            arguments: null,
            condition: Economist__errorsClientPointerField__resolver_reader,
            isUpdatable: false,
            refetchQueryIndex: 0,
            selections: [
              {
                kind: "Scalar",
                isFallible: false,
                fieldName: "id",
                alias: null,
                arguments: null,
                isUpdatable: false,
              },
            ],
          },
        ],
      },
    ],
  },
];

const artifact = (): EagerReaderArtifact<
  Query__errorsClientPointer__param,
  Query__errorsClientPointer__output_type
> => ({
  kind: "EagerReaderArtifact",
  fieldName: "errorsClientPointer",
  resolver,
  readerAst,
  hasUpdatable: false,
});

export default artifact;
