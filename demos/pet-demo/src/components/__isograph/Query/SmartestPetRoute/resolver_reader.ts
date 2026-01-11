import type {ComponentReaderArtifact, ExtractSecondParam, ReaderAst } from '@isograph/react';
import { Query__SmartestPetRoute__param } from './param_type';
import { SmartestPetRoute as resolver } from '../../../SmartestPet';
import Pet__Avatar__resolver_reader from '../../Pet/Avatar/resolver_reader';
import Pet__checkinsPointer__resolver_reader from '../../Pet/checkinsPointer/resolver_reader';
import Pet__fullName__resolver_reader from '../../Pet/fullName/resolver_reader';
import Query__smartestPet__resolver_reader from '../../Query/smartestPet/resolver_reader';

const readerAst: ReaderAst<Query__SmartestPetRoute__param> = [
  {
    kind: "Linked",
    isFallible: false,
    fieldName: "smartestPet",
    alias: null,
    arguments: null,
    condition: Query__smartestPet__resolver_reader,
    isUpdatable: false,
    refetchQueryIndex: 0,
    selections: [
      {
        kind: "Scalar",
        isFallible: false,
        fieldName: "id",
        alias: null,
        arguments: null,
        isUpdatable: false,
      },
      {
        kind: "Resolver",
        alias: "fullName",
        arguments: null,
        readerArtifact: Pet__fullName__resolver_reader,
        usedRefetchQueries: [],
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
        isFallible: true,
        fieldName: "stats",
        alias: null,
        arguments: null,
        condition: null,
        isUpdatable: false,
        refetchQueryIndex: null,
        selections: [
          {
            kind: "Scalar",
            isFallible: true,
            fieldName: "intelligence",
            alias: null,
            arguments: null,
            isUpdatable: false,
          },
        ],
      },
      {
        kind: "Scalar",
        isFallible: false,
        fieldName: "picture",
        alias: null,
        arguments: null,
        isUpdatable: false,
      },
      {
        kind: "Linked",
        isFallible: false,
        fieldName: "checkinsPointer",
        alias: null,
        arguments: null,
        condition: Pet__checkinsPointer__resolver_reader,
        isUpdatable: false,
        refetchQueryIndex: 1,
        selections: [
          {
            kind: "Scalar",
            isFallible: false,
            fieldName: "location",
            alias: null,
            arguments: null,
            isUpdatable: false,
          },
        ],
      },
    ],
  },
];

const artifact = (): ComponentReaderArtifact<
  Query__SmartestPetRoute__param,
  ExtractSecondParam<typeof resolver>
> => ({
  kind: "ComponentReaderArtifact",
  fieldName: "SmartestPetRoute",
  resolver,
  readerAst,
  hasUpdatable: false,
});

export default artifact;
