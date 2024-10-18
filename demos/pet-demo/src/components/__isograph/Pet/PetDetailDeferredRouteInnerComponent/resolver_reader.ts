import type {ComponentReaderArtifact, ExtractSecondParam, ReaderAst } from '@isograph/react';
import { Pet__PetDetailDeferredRouteInnerComponent__param } from './param_type';
import { PetDetailDeferredRouteInnerComponent as resolver } from '../../../PetDetailDeferredRoute';
import Pet__PetCheckinsCard__entrypoint from '../../Pet/PetCheckinsCard/entrypoint';

const readerAst: ReaderAst<Pet__PetDetailDeferredRouteInnerComponent__param> = [
  {
    kind: "Scalar",
    fieldName: "name",
    alias: null,
    arguments: null,
  },
  {
    kind: "LoadablySelectedField",
    alias: "PetCheckinsCard",
    name: "PetCheckinsCard",
    queryArguments: null,
    refetchReaderAst: [
      {
        kind: "Scalar",
        fieldName: "id",
        alias: null,
        arguments: null,
      },
    ],
    entrypoint: Pet__PetCheckinsCard__entrypoint,
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
