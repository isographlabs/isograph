import type {ComponentReaderArtifact, ExtractSecondParam, ReaderAst } from '@isograph/react';
import { Query__SmartestPetRoute__param } from './param_type';
import { SmartestPetRoute as resolver } from '../../../SmartestPet';
import Pet__Avatar__resolver_reader from '../../Pet/Avatar/resolver_reader';
import Query__smartestPet__resolver_reader from '../../Query/smartestPet/resolver_reader';

const readerAst: ReaderAst<Query__SmartestPetRoute__param> = [
  {
    kind: "Linked",
    fieldName: "smartestPet",
    alias: null,
    arguments: null,
    condition: Query__smartestPet__resolver_reader,
    isUpdatable: false,
    selections: [
      {
        kind: "Scalar",
        fieldName: "id",
        alias: null,
        arguments: null,
        isUpdatable: false,
      },
      {
        kind: "Scalar",
        fieldName: "name",
        alias: null,
        arguments: null,
        isUpdatable: false,
      },
      {
        kind: "Resolver",
        alias: "Avatar",
        arguments: null,
        readerArtifact: Pet__Avatar__resolver_reader,
        usedRefetchQueries: [],
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
      {
        kind: "Scalar",
        fieldName: "picture",
        alias: null,
        arguments: null,
        isUpdatable: false,
      },
    ],
    refetchQueryIndex: 0,
  },
];

const artifact: ComponentReaderArtifact<
  Query__SmartestPetRoute__param,
  ExtractSecondParam<typeof resolver>
> = {
  kind: "ComponentReaderArtifact",
  fieldName: "Query.SmartestPetRoute",
  resolver,
  readerAst,
  hasUpdatable: false,
};

export default artifact;
