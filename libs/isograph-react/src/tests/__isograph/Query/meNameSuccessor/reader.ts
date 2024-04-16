import type {ReaderArtifact, ReaderAst} from '@isograph/react';
import { Query__meNameSuccessor__param } from './param_type.ts';
import { Query__meNameSuccessor__outputType } from './output_type.ts';
import { meNameField as resolver } from '../../../meNameSuccessor.ts';

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

const artifact: ReaderArtifact<
  Query__meNameSuccessor__param,
  Query__meNameSuccessor__outputType
> = {
  kind: "ReaderArtifact",
  fieldName: "meNameSuccessor",
  resolver: resolver as any,
  readerAst,
  variant: { kind: "Eager" },
};

export default artifact;
