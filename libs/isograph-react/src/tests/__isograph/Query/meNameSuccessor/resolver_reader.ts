import type { EagerReaderArtifact, ReaderAst } from '@isograph/react';
import { Query__meNameSuccessor__param } from './param_type';
import { Query__meNameSuccessor__output_type } from './output_type';
import { meNameField as resolver } from '../../../meNameSuccessor';

const readerAst: ReaderAst<Query__meNameSuccessor__param> = [
  {
    kind: "Linked",
    fieldName: "me",
    alias: null,
    arguments: null,
    condition: null,
    isUpdatable: false,
    refetchQueryIndex: null,
    selections: [
      {
        kind: "Scalar",
        fieldName: "name",
        alias: null,
        arguments: null,
        isUpdatable: false,
      },
      {
        kind: "Linked",
        fieldName: "successor",
        alias: null,
        arguments: null,
        condition: null,
        isUpdatable: false,
        refetchQueryIndex: null,
        selections: [
          {
            kind: "Linked",
            fieldName: "successor",
            alias: null,
            arguments: null,
            condition: null,
            isUpdatable: false,
            refetchQueryIndex: null,
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
        ],
      },
    ],
  },
];

const artifact: EagerReaderArtifact<
  Query__meNameSuccessor__param,
  Query__meNameSuccessor__output_type
> = {
  kind: "EagerReaderArtifact",
  fieldName: "Query.meNameSuccessor",
  resolver,
  readerAst,
  hasUpdatable: false,
};

export default artifact;
