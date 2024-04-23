import type {ComponentReaderArtifact, ExtractSecondParam, ReaderAst, RefetchQueryNormalizationArtifact} from '@isograph/react';
import { Query__PetDetailRoute__param } from './param_type';
import { PetDetailRoute as resolver } from '../../../PetDetailRoute.tsx';
import Pet__PetBestFriendCard from '../../Pet/PetBestFriendCard/reader';
import Pet__PetCheckinsCard from '../../Pet/PetCheckinsCard/reader';
import Pet__PetPhraseCard from '../../Pet/PetPhraseCard/reader';
import Pet__PetTaglineCard from '../../Pet/PetTaglineCard/reader';

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
        readerArtifact: Pet__PetCheckinsCard,
        usedRefetchQueries: [2, ],
      },
      {
        kind: "Resolver",
        alias: "PetBestFriendCard",
        arguments: null,
        readerArtifact: Pet__PetBestFriendCard,
        usedRefetchQueries: [0, 1, ],
      },
      {
        kind: "Resolver",
        alias: "PetPhraseCard",
        arguments: null,
        readerArtifact: Pet__PetPhraseCard,
        usedRefetchQueries: [],
      },
      {
        kind: "Resolver",
        alias: "PetTaglineCard",
        arguments: null,
        readerArtifact: Pet__PetTaglineCard,
        usedRefetchQueries: [],
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
