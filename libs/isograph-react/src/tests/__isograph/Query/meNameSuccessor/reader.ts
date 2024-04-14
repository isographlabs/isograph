import type {ReaderArtifact, ReaderAst, ExtractSecondParam} from '@isograph/react';
import { meNameField as resolver } from '../../../meNameSuccessor.ts';

// the type, when read out (either via useLazyReference or via graph)
export type Query__meNameSuccessor__outputType = ReturnType<typeof resolver>;

const readerAst: ReaderAst<Query__meNameSuccessor__param> = [
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
      {
        kind: "Linked",
        fieldName: "successor",
        alias: null,
        arguments: null,
        selections: [
          {
            kind: "Linked",
            fieldName: "successor",
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
        ],
      },
    ],
  },
];

export type Query__meNameSuccessor__param = {
  me: {
    name: string,
    successor: ({
      successor: ({
        name: string,
      } | null),
    } | null),
  },
};

const artifact: ReaderArtifact<
  Query__meNameSuccessor__param,
  Query__meNameSuccessor__outputType
> = {
  kind: "ReaderArtifact",
  resolver: resolver as any,
  readerAst,
  variant: { kind: "Eager" },
};

export default artifact;
