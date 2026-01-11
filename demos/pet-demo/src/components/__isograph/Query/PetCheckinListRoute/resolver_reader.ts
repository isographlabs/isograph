import type {ComponentReaderArtifact, ExtractSecondParam, ReaderAst } from '@isograph/react';
import { Query__PetCheckinListRoute__param } from './param_type';
import { PetDetailDeferredRouteComponent as resolver } from '../../../PetCheckinListRoute';
import Pet__FirstCheckinMakeSuperButton__resolver_reader from '../../Pet/FirstCheckinMakeSuperButton/resolver_reader';
import Pet__fullName__resolver_reader from '../../Pet/fullName/resolver_reader';

const readerAst: ReaderAst<Query__PetCheckinListRoute__param> = [
  {
    kind: "Linked",
    isFallible: true,
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
    refetchQueryIndex: null,
    selections: [
      {
        kind: "Resolver",
        alias: "FirstCheckinMakeSuperButton",
        arguments: null,
        readerArtifact: Pet__FirstCheckinMakeSuperButton__resolver_reader,
        usedRefetchQueries: [0, ],
      },
      {
        kind: "Resolver",
        alias: "fullName",
        arguments: null,
        readerArtifact: Pet__fullName__resolver_reader,
        usedRefetchQueries: [],
      },
      {
        kind: "LoadablySelectedField",
        alias: "PetCheckinsCardList",
        name: "PetCheckinsCardList",
        queryArguments: null,
        refetchReaderAst: [
          {
            kind: "Scalar",
            isFallible: false,
            fieldName: "id",
            alias: null,
            arguments: null,
            isUpdatable: false,
          },
        ],
        entrypoint: {
          kind: "EntrypointLoader",
          typeAndField: "Pet__PetCheckinsCardList",
          readerArtifactKind: "EagerReaderArtifact",
          loader: () => import("../../Pet/PetCheckinsCardList/entrypoint").then(module => module.default),
        },
      },
    ],
  },
];

const artifact = (): ComponentReaderArtifact<
  Query__PetCheckinListRoute__param,
  ExtractSecondParam<typeof resolver>
> => ({
  kind: "ComponentReaderArtifact",
  fieldName: "PetCheckinListRoute",
  resolver,
  readerAst,
  hasUpdatable: false,
});

export default artifact;
