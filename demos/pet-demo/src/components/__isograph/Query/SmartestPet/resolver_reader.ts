import type { EagerReaderArtifact, ReaderAst } from '@isograph/react';
import { Query__smartestPet__param } from './param_type';
import { Query__smartestPet__output_type } from './output_type';
import { SmartestPet as resolver } from '../../../SmartestPet';

const readerAst: ReaderAst<Query__smartestPet__param> = [
  {
    kind: "Linked",
    fieldName: "pets",
    alias: null,
    arguments: null,
    condition: null,
    isUpdatable: false,
    selections: [
      {
        kind: "Link",
        alias: "link",
      },
      {
        kind: "Linked",
        fieldName: "stats",
        alias: null,
        arguments: null,
        condition: null,
        isUpdatable: false,
        selections: [
          {
            kind: "Scalar",
            fieldName: "intelligence",
            alias: null,
            arguments: null,
            isUpdatable: false,
          },
        ],
        refetchQueryIndex: null,
      },
    ],
    refetchQueryIndex: null,
  },
];

const artifact: EagerReaderArtifact<
  Query__smartestPet__param,
  Query__smartestPet__output_type
> = {
  kind: "EagerReaderArtifact",
  fieldName: "Query.smartestPet",
  resolver,
  readerAst,
  hasUpdatable: false,
};

export default artifact;
