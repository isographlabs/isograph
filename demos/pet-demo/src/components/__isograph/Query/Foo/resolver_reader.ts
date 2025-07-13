import type { EagerReaderArtifact, ReaderAst } from '@isograph/react';
import { Query__Foo__param } from './param_type';
import { Query__Foo__output_type } from './output_type';
import { foo as resolver } from '../../../foo';

const readerAst: ReaderAst<Query__Foo__param> = [
  {
    kind: "Linked",
    fieldName: "pet",
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
        kind: "Scalar",
        fieldName: "name",
        alias: null,
        arguments: null,
        isUpdatable: false,
      },
    ],
    refetchQueryIndex: null,
  },
];

const artifact: EagerReaderArtifact<
  Query__Foo__param,
  Query__Foo__output_type
> = {
  kind: "EagerReaderArtifact",
  fieldName: "Query.Foo",
  resolver,
  readerAst,
  hasUpdatable: false,
};

export default artifact;
