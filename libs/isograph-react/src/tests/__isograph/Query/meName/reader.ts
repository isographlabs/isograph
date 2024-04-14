import type {ReaderArtifact, ReaderAst, ExtractSecondParam} from '@isograph/react';
import { meNameField as resolver } from '../../../garbageCollection.test.ts';

// the type, when read out (either via useLazyReference or via graph)
export type Query__meName__outputType = ReturnType<typeof resolver>;

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

export type Query__meName__param = {
  me: {
    name: string,
  },
};

const artifact: ReaderArtifact<
  Query__meName__param,
  Query__meName__outputType
> = {
  kind: "ReaderArtifact",
  resolver: resolver as any,
  readerAst,
  variant: { kind: "Eager" },
};

export default artifact;
