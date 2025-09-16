import type {ComponentReaderArtifact, ExtractSecondParam, ReaderAst } from '@isograph/react';
import { Query__Random__param } from './param_type';
import { Random as resolver } from '../../../RandomLoader';
import Node__asPet__resolver_reader from '../../Node/asPet/resolver_reader';
import Query__firstNode__resolver_reader from '../../Query/firstNode/resolver_reader';

const readerAst: ReaderAst<Query__Random__param> = [
  {
    kind: "Linked",
    fieldName: "firstNode",
    alias: null,
    arguments: null,
    condition: Query__firstNode__resolver_reader,
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
            kind: "Scalar",
            fieldName: "nickname",
            alias: null,
            arguments: null,
            isUpdatable: false,
          },
        ],
        refetchQueryIndex: null,
      },
    ],
    refetchQueryIndex: 0,
  },
];

const artifact: ComponentReaderArtifact<
  Query__Random__param,
  ExtractSecondParam<typeof resolver>
> = {
  kind: "ComponentReaderArtifact",
  fieldName: "Query.Random",
  resolver,
  readerAst,
  hasUpdatable: false,
};

export default artifact;
