import type {ComponentReaderArtifact, ExtractSecondParam, ReaderAst } from '@isograph/react';
import { Query__PetByName__param } from './param_type';
import { PetByNameRouteComponent as resolver } from '../../../PetByName';
import Pet__PetDetailDeferredRouteInnerComponent__resolver_reader from '../../Pet/PetDetailDeferredRouteInnerComponent/resolver_reader';

const readerAst: ReaderAst<Query__PetByName__param> = [
  {
    kind: "Linked",
    fieldName: "petByName",
    alias: "pet",
    arguments: [
      [
        "name",
        { kind: "Variable", name: "name" },
      ],
    ],
    selections: [
      {
        kind: "Resolver",
        alias: "PetDetailDeferredRouteInnerComponent",
        arguments: null,
        readerArtifact: Pet__PetDetailDeferredRouteInnerComponent__resolver_reader,
        usedRefetchQueries: [],
      },
    ],
  },
];

const artifact: ComponentReaderArtifact<
  Query__PetByName__param,
  ExtractSecondParam<typeof resolver>
> = {
  kind: "ComponentReaderArtifact",
  componentName: "Query.PetByName",
  resolver,
  readerAst,
};

export default artifact;
