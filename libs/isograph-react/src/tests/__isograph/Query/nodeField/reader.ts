import type {ReaderArtifact, ReaderAst, ExtractSecondParam} from '@isograph/react';
import { nodeField as resolver } from '../../../nodeQuery.ts';

// the type, when read out (either via useLazyReference or via graph)
export type Query__nodeField__outputType = ReturnType<typeof resolver>;

const readerAst: ReaderAst<Query__nodeField__param> = [
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
    selections: [
      {
        kind: "Scalar",
        fieldName: "id",
        alias: null,
        arguments: null,
      },
    ],
  },
];

export type Query__nodeField__param = {
  node: ({
    id: string,
  } | null),
};

const artifact: ReaderArtifact<
  Query__nodeField__param,
  Query__nodeField__outputType
> = {
  kind: "ReaderArtifact",
  resolver: resolver as any,
  readerAst,
  variant: { kind: "Eager" },
};

export default artifact;
