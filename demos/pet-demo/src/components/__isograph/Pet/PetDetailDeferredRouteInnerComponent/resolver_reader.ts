import type {ComponentReaderArtifact, ExtractSecondParam, ReaderAst } from '@isograph/react';
import { Pet__PetDetailDeferredRouteInnerComponent__param } from './param_type';
import { PetDetailDeferredRouteInnerComponent as resolver } from '../../../PetDetailDeferredRoute';
import Pet__PetCheckinsCard__resolver_reader from '../../Pet/PetCheckinsCard/resolver_reader';
import Pet__PetCheckinsCard__refetch_reader from '../../Pet/PetCheckinsCard/refetch_reader';

const readerAst: ReaderAst<Pet__PetDetailDeferredRouteInnerComponent__param> = [
  {
    kind: "Scalar",
    fieldName: "name",
    alias: null,
    arguments: null,
  },
  {
    kind: "ImperativelyLoadedField",
    alias: "PetCheckinsCard",
    refetchReaderArtifact: Pet__PetCheckinsCard__refetch_reader,
    resolverReaderArtifact: Pet__PetCheckinsCard__resolver_reader,
    refetchQuery: 0,
    name: "PetCheckinsCard",
    usedRefetchQueries: [1, ],
  },
];

const artifact: ComponentReaderArtifact<
  Pet__PetDetailDeferredRouteInnerComponent__param,
  ExtractSecondParam<typeof resolver>
> = {
  kind: "ComponentReaderArtifact",
  componentName: "Pet.PetDetailDeferredRouteInnerComponent",
  resolver,
  readerAst,
};

export default artifact;
