import type {ComponentReaderArtifact, ExtractSecondParam, ReaderAst } from '@isograph/react';
import { Query__PetDetailDeferredRoute__param } from './param_type';
import { PetDetailDeferredRouteComponent as resolver } from '../../../PetDetailDeferredRoute';
import Pet__PetDetailDeferredRouteInnerComponent__resolver_reader from '../../Pet/PetDetailDeferredRouteInnerComponent/resolver_reader';

const readerAst: ReaderAst<Query__PetDetailDeferredRoute__param> = [
  {
    kind: "Linked",
    fieldName: "pet",
    alias: null,
    arguments: [
      [
        "id",
        { kind: "Variable", name: "id" },
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
  Query__PetDetailDeferredRoute__param,
  ExtractSecondParam<typeof resolver>
> = {
  kind: "ComponentReaderArtifact",
  componentName: "Query.PetDetailDeferredRoute",
  resolver,
  readerAst,
};

export default artifact;
