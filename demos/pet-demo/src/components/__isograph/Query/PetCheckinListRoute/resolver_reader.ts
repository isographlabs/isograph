import type {ComponentReaderArtifact, ExtractSecondParam, ReaderAst } from '@isograph/react';
import { Query__PetCheckinListRoute__param } from './param_type';
import { PetDetailDeferredRouteComponent as resolver } from '../../../PetCheckinListRoute';
import Pet__FirstCheckinMakeSuperButton__resolver_reader from '../../Pet/FirstCheckinMakeSuperButton/resolver_reader';

const readerAst: ReaderAst<Query__PetCheckinListRoute__param> = [
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
    condition: null,
    isUpdatable: false,
    selections: [
      {
        kind: "Resolver",
        alias: "FirstCheckinMakeSuperButton",
        arguments: null,
        readerArtifact: Pet__FirstCheckinMakeSuperButton__resolver_reader,
        usedRefetchQueries: [0, ],
      },
      {
        kind: "Scalar",
        fieldName: "name",
        alias: null,
        arguments: null,
        isUpdatable: false,
      },
      {
        kind: "LoadablySelectedField",
        alias: "PetCheckinsCardList",
        name: "PetCheckinsCardList",
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
        entrypoint: { 
          kind: "EntrypointLoader",
          typeAndField: "Pet__PetCheckinsCardList",
          loader: () => import("../../Pet/PetCheckinsCardList/entrypoint").then(module => module.default),
        },
      },
    ],
    refetchQueryIndex: null,
  },
];

const artifact: ComponentReaderArtifact<
  Query__PetCheckinListRoute__param,
  ExtractSecondParam<typeof resolver>
> = {
  kind: "ComponentReaderArtifact",
  fieldName: "Query.PetCheckinListRoute",
  resolver,
  readerAst,
  hasUpdatable: false,
};

export default artifact;
