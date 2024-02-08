import type {ReaderArtifact, ReaderAst} from '@isograph/react';
import { PetDetailRoute as resolver } from '../../../PetDetailRoute.tsx';
import Pet__PetBestFriendCard, { ReadOutType as Pet__PetBestFriendCard__outputType } from '../../Pet/PetBestFriendCard/reader';
import Pet__PetCheckinsCard, { ReadOutType as Pet__PetCheckinsCard__outputType } from '../../Pet/PetCheckinsCard/reader';
import Pet__PetPhraseCard, { ReadOutType as Pet__PetPhraseCard__outputType } from '../../Pet/PetPhraseCard/reader';
import Pet__PetTaglineCard, { ReadOutType as Pet__PetTaglineCard__outputType } from '../../Pet/PetTaglineCard/reader';

// the type, when read out (either via useLazyReference or via graph)
export type ReadOutType = (React.FC<any>);

export type ReadFromStoreType = ResolverParameterType;

const readerAst: ReaderAst<ReadFromStoreType> = [
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
        usedRefetchQueries: [],
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

export type ResolverParameterType = { data:
{
  pet: ({
    name: string,
    PetCheckinsCard: Pet__PetCheckinsCard__outputType,
    PetBestFriendCard: Pet__PetBestFriendCard__outputType,
    PetPhraseCard: Pet__PetPhraseCard__outputType,
    PetTaglineCard: Pet__PetTaglineCard__outputType,
  } | null),
},
[index: string]: any };

// The type, when returned from the resolver
export type ResolverReturnType = ReturnType<typeof resolver>;

const artifact: ReaderArtifact<ReadFromStoreType, ResolverParameterType, ReadOutType> = {
  kind: "ReaderArtifact",
  resolver: resolver as any,
  readerAst,
  variant: { kind: "Component", componentName: "Query.PetDetailRoute" },
};

export default artifact;
