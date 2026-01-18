import type {ComponentReaderArtifact, ExtractSecondParam, ReaderAst } from '@isograph/react';
import { Query__PetNameList__param } from './param_type';
import { PetNameList as resolver } from '../../../RestEndpointDemoRoute';
import Pet__fullName__resolver_reader from '../../Pet/fullName/resolver_reader';

const readerAst: ReaderAst<Query__PetNameList__param> = [
  {
    kind: "Linked",
    isFallible: false,
    fieldName: "pets",
    alias: null,
    arguments: null,
    condition: null,
    isUpdatable: false,
    refetchQueryIndex: null,
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
    ],
  },
];

const artifact = (): ComponentReaderArtifact<
  Query__PetNameList__param,
  ExtractSecondParam<typeof resolver>
> => ({
  kind: "ComponentReaderArtifact",
  fieldName: "PetNameList",
  resolver,
  readerAst,
  hasUpdatable: false,
});

export default artifact;
