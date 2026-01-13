import type {ComponentReaderArtifact, ExtractSecondParam, ReaderAst } from '@isograph/react';
import { Pet__PetDetailDeferredRouteInnerComponent__param } from './param_type';
import { PetDetailDeferredRouteInnerComponent as resolver } from '../../../PetDetailDeferredRoute';
import Pet__PetCheckinsCard__entrypoint from '../../Pet/PetCheckinsCard/entrypoint';
import Pet__fullName__resolver_reader from '../../Pet/fullName/resolver_reader';

const readerAst: ReaderAst<Pet__PetDetailDeferredRouteInnerComponent__param> = [
  {
    kind: "Resolver",
    alias: "fullName",
    arguments: null,
    readerArtifact: Pet__fullName__resolver_reader,
    usedRefetchQueries: [],
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
        isUpdatable: false,
      },
    ],
    entrypoint: Pet__PetCheckinsCard__entrypoint,
  },
];

const artifact = (): ComponentReaderArtifact<
  Pet__PetDetailDeferredRouteInnerComponent__param,
  ExtractSecondParam<typeof resolver>
> => ({
  kind: "ComponentReaderArtifact",
  fieldName: "PetDetailDeferredRouteInnerComponent",
  resolver,
  readerAst,
  hasUpdatable: false,
});

export default artifact;
