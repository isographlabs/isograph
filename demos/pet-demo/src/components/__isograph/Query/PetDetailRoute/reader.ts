import type {ReaderArtifact, ReaderAst, ExtractSecondParam} from '@isograph/react';
import { PetDetailRoute as resolver } from '../../../PetDetailRoute.tsx';
import Pet__PetBestFriendCard, { Pet__PetBestFriendCard__outputType} from '../../Pet/PetBestFriendCard/reader';
import Pet__PetCheckinsCard, { Pet__PetCheckinsCard__outputType} from '../../Pet/PetCheckinsCard/reader';
import Pet__PetPhraseCard, { Pet__PetPhraseCard__outputType} from '../../Pet/PetPhraseCard/reader';
import Pet__PetTaglineCard, { Pet__PetTaglineCard__outputType} from '../../Pet/PetTaglineCard/reader';

// the type, when read out (either via useLazyReference or via graph)
export type Query__PetDetailRoute__outputType = (React.FC<ExtractSecondParam<typeof resolver>>);

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
        usedRefetchQueries: [0, ],
      },
      {
        kind: "Resolver",
        alias: "PetBestFriendCard",
        arguments: null,
        readerArtifact: Pet__PetBestFriendCard,
        usedRefetchQueries: [1, 2, ],
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

export type Query__PetDetailRoute__param = {
  pet: ({
    name: string,
    PetCheckinsCard: Pet__PetCheckinsCard__outputType,
    PetBestFriendCard: Pet__PetBestFriendCard__outputType,
    PetPhraseCard: Pet__PetPhraseCard__outputType,
    PetTaglineCard: Pet__PetTaglineCard__outputType,
  } | null),
};

const artifact: ReaderArtifact<
  Query__PetDetailRoute__param,
  Query__PetDetailRoute__outputType
> = {
  kind: "ReaderArtifact",
  resolver: resolver as any,
  readerAst,
  variant: { kind: "Component", componentName: "Query.PetDetailRoute" },
};

export default artifact;
