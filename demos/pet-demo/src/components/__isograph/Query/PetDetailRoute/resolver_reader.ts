import type {ComponentReaderArtifact, ExtractSecondParam, ReaderAst } from '@isograph/react';
import { Query__PetDetailRoute__param } from './param_type';
import { PetDetailRouteComponent as resolver } from '../../../PetDetailRoute';
import Pet__PetBestFriendCard__resolver_reader from '../../Pet/PetBestFriendCard/resolver_reader';
import Pet__PetCheckinsCard__resolver_reader from '../../Pet/PetCheckinsCard/resolver_reader';
import Pet__PetPhraseCard__resolver_reader from '../../Pet/PetPhraseCard/resolver_reader';
import Pet__PetStatsCard__resolver_reader from '../../Pet/PetStatsCard/resolver_reader';
import Pet__PetTaglineCard__resolver_reader from '../../Pet/PetTaglineCard/resolver_reader';
import Pet__custom_pet_refetch__refetch_reader from '../../Pet/custom_pet_refetch/refetch_reader';

const readerAst: ReaderAst<Query__PetDetailRoute__param> = [
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
        kind: "ImperativelyLoadedField",
        alias: "custom_pet_refetch",
        refetchReaderArtifact: Pet__custom_pet_refetch__refetch_reader,
        refetchQueryIndex: 1,
        name: "custom_pet_refetch",
      },
      {
        kind: "Scalar",
        fieldName: "name",
        alias: null,
        arguments: null,
        isUpdatable: false,
      },
      {
        kind: "Resolver",
        alias: "PetCheckinsCard",
        arguments: null,
        readerArtifact: Pet__PetCheckinsCard__resolver_reader,
        usedRefetchQueries: [4, ],
      },
      {
        kind: "Resolver",
        alias: "PetBestFriendCard",
        arguments: null,
        readerArtifact: Pet__PetBestFriendCard__resolver_reader,
        usedRefetchQueries: [0, 2, 3, ],
      },
      {
        kind: "Resolver",
        alias: "PetPhraseCard",
        arguments: null,
        readerArtifact: Pet__PetPhraseCard__resolver_reader,
        usedRefetchQueries: [],
      },
      {
        kind: "Resolver",
        alias: "PetTaglineCard",
        arguments: null,
        readerArtifact: Pet__PetTaglineCard__resolver_reader,
        usedRefetchQueries: [],
      },
      {
        kind: "Resolver",
        alias: "PetStatsCard",
        arguments: [
          [
            "id",
            { kind: "Variable", name: "id" },
          ],
        ],
        readerArtifact: Pet__PetStatsCard__resolver_reader,
        usedRefetchQueries: [5, ],
      },
    ],
    refetchQueryIndex: null,
  },
];

const artifact: ComponentReaderArtifact<
  Query__PetDetailRoute__param,
  ExtractSecondParam<typeof resolver>
> = {
  kind: "ComponentReaderArtifact",
  fieldName: "Query.PetDetailRoute",
  resolver,
  readerAst,
  hasUpdatable: false,
};

export default artifact;
