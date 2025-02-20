import type { EagerReaderArtifact, ReaderAst } from '@isograph/react';
import { Query__startUpdate__param } from './param_type';
import { Query__startUpdate__output_type } from './output_type';
import { startUpdate as resolver } from '../../../startUpdate.test';
import Economist__asEconomist__resolver_reader from '../../Economist/asEconomist/resolver_reader';
import Economist__omitted__resolver_reader from '../../Economist/omitted/resolver_reader';

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
    selections: [
      {
        kind: "Linked",
        fieldName: "asEconomist",
        alias: null,
        arguments: null,
        condition: Economist__asEconomist__resolver_reader,
        selections: [
          {
            kind: "Resolver",
            alias: "omitted",
            arguments: null,
            readerArtifact: Economist__omitted__resolver_reader,
            usedRefetchQueries: [],
          },
          {
            kind: "Scalar",
            fieldName: "id",
            alias: null,
            arguments: null,
            isUpdatable: false,
          },
          {
            kind: "Scalar",
            fieldName: "name",
            alias: null,
            arguments: null,
            isUpdatable: true,
          },
          {
            kind: "Linked",
            fieldName: "successor",
            alias: null,
            arguments: null,
            condition: null,
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
                fieldName: "name",
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
