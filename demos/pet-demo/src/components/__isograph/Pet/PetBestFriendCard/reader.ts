import type {ReaderArtifact, ReaderAst} from '@isograph/react';
import { PetBestFriendCard as resolver } from '../../../PetBestFriendCard.tsx';
import Pet__PetUpdater, { Pet__PetUpdater__outputType} from '../PetUpdater/reader';

// the type, when read out (either via useLazyReference or via graph)
export type Pet__PetBestFriendCard__outputType = (React.FC<any>);

const readerAst: ReaderAst<Pet__PetBestFriendCard__param> = [
  {
    kind: "Scalar",
    fieldName: "id",
    alias: null,
    arguments: null,
  },
  {
    kind: "Resolver",
    alias: "PetUpdater",
    arguments: null,
    readerArtifact: Pet__PetUpdater,
    usedRefetchQueries: [0, 1, ],
  },
  {
    kind: "Linked",
    fieldName: "best_friend_relationship",
    alias: null,
    arguments: null,
    selections: [
      {
        kind: "Scalar",
        fieldName: "picture_together",
        alias: null,
        arguments: null,
      },
      {
        kind: "Linked",
        fieldName: "best_friend",
        alias: null,
        arguments: null,
        selections: [
          {
            kind: "Scalar",
            fieldName: "id",
            alias: null,
            arguments: null,
          },
          {
            kind: "Scalar",
            fieldName: "name",
            alias: null,
            arguments: null,
          },
          {
            kind: "Scalar",
            fieldName: "picture",
            alias: null,
            arguments: null,
          },
        ],
      },
    ],
  },
];

export type Pet__PetBestFriendCard__param = { data:
{
  id: string,
  PetUpdater: Pet__PetUpdater__outputType,
  best_friend_relationship: ({
    picture_together: (string | null),
    best_friend: {
      id: string,
      name: string,
      picture: string,
    },
  } | null),
},
[index: string]: any };

const artifact: ReaderArtifact<
  Pet__PetBestFriendCard__param,
  Pet__PetBestFriendCard__param,
  Pet__PetBestFriendCard__outputType
> = {
  kind: "ReaderArtifact",
  resolver: resolver as any,
  readerAst,
  variant: { kind: "Component", componentName: "Pet.PetBestFriendCard" },
};

export default artifact;
