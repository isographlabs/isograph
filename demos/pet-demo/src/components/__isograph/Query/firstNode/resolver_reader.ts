import type { EagerReaderArtifact, ReaderAst } from '@isograph/react';
import { Query__firstNode__param } from './param_type';
import { Query__firstNode__output_type } from './output_type';
import { firstNode as resolver } from '../../../Query/firstNode';
import Node__asPet__resolver_reader from '../../Node/asPet/resolver_reader';

const readerAst: ReaderAst<Query__firstNode__param> = [
  {
    kind: "Linked",
    fieldName: "node",
    alias: null,
    arguments: [
      [
        "id",
        { kind: "Literal", value: 0 },
      ],
    ],
    condition: null,
    isUpdatable: false,
    selections: [
      {
        kind: "Linked",
        fieldName: "asPet",
        alias: null,
        arguments: null,
        condition: Node__asPet__resolver_reader,
        isUpdatable: false,
        selections: [
          {
            kind: "Link",
            alias: "link",
          },
        ],
        refetchQueryIndex: null,
      },
    ],
    refetchQueryIndex: null,
  },
];

const artifact: EagerReaderArtifact<
  Query__firstNode__param,
  Query__firstNode__output_type
> = {
  kind: "EagerReaderArtifact",
  fieldName: "Query.firstNode",
  resolver,
  readerAst,
  hasUpdatable: false,
};

export default artifact;
