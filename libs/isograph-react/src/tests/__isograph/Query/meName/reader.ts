import type {ReaderArtifact, ReaderAst} from '@isograph/react';
import { Query__meName__param } from './param_type.ts';
import { Query__meName__outputType } from './output_type.ts';
import { meNameField as resolver } from '../../../garbageCollection.test.ts';

const readerAst: ReaderAst<Query__meName__param> = [
  {
    kind: "Linked",
    fieldName: "me",
    alias: null,
    arguments: null,
    selections: [
      {
        kind: "Scalar",
        fieldName: "name",
        alias: null,
        arguments: null,
      },
    ],
  },
];

const artifact: ReaderArtifact<
  Query__meName__param,
  Query__meName__outputType
> = {
  kind: "ReaderArtifact",
  fieldName: "meName",
  resolver: resolver as any,
  readerAst,
  variant: { kind: "Eager" },
};

export default artifact;
