import type {ComponentReaderArtifact, ExtractSecondParam, ReaderAst } from '@isograph/react';
import { Query__PetDetailRoute__param } from './param_type';
import { PetDetailRouteComponent as resolver } from '../../../PetDetailRoute';
import Pet__PetBestFriendCard__resolver_reader from '../../Pet/PetBestFriendCard/resolver_reader';
import Pet__PetCheckinsCard__resolver_reader from '../../Pet/PetCheckinsCard/resolver_reader';
import Pet__PetPhraseCard__resolver_reader from '../../Pet/PetPhraseCard/resolver_reader';
import Pet__PetStatsCard__resolver_reader from '../../Pet/PetStatsCard/resolver_reader';
import Pet__PetTaglineCard__resolver_reader from '../../Pet/PetTaglineCard/resolver_reader';

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
        usedRefetchQueries: [3, ],
      },
      {
        kind: "Resolver",
        alias: "PetBestFriendCard",
        arguments: null,
        readerArtifact: Pet__PetBestFriendCard__resolver_reader,
        usedRefetchQueries: [0, 1, 2, ],
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
        usedRefetchQueries: [4, ],
      },
    ],
  },
];

const artifact: ComponentReaderArtifact<
  Query__PetDetailRoute__param,
  ExtractSecondParam<typeof resolver>
> = {
  kind: "ComponentReaderArtifact",
  componentName: "Query.PetDetailRoute",
  resolver,
  readerAst,
};

export default artifact;
