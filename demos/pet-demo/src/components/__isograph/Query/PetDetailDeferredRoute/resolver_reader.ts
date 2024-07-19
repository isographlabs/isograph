import type {ComponentReaderArtifact, ExtractSecondParam, ReaderAst } from '@isograph/react';
import { Query__PetDetailDeferredRoute__param } from './param_type';
import { PetDetailDeferredRoute as resolver } from '../../../PetDetailDeferredRoute';
import Pet__PetCheckinsCard__resolver_reader from '../../Pet/PetCheckinsCard/resolver_reader';

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
        kind: "Scalar",
        fieldName: "name",
        alias: null,
        arguments: null,
      },
      {
        kind: "Resolver",
        alias: "PetCheckinsCard",
        arguments: null,
        readerArtifact: Pet__PetCheckinsCard__resolver_reader,
        usedRefetchQueries: [0, ],
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
