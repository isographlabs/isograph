import type { EagerReaderArtifact, ReaderAst } from '@isograph/react';
import { Query__meName__param } from './param_type';
import { Query__meName__output_type } from './output_type';
import { meNameField as resolver } from '../../../garbageCollection.test';

const readerAst: ReaderAst<Query__meName__param> = [
  {
    kind: "Linked",
    fieldName: "me",
    alias: null,
    arguments: null,
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
  },
];

const artifact: EagerReaderArtifact<
  Query__meName__param,
  Query__meName__output_type
> = {
  kind: "EagerReaderArtifact",
  fieldName: "Query.meName",
  resolver,
  readerAst,
  hasUpdatable: false,
};

export default artifact;
