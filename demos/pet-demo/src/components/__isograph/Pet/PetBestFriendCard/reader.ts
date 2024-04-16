import type {ReaderArtifact, ReaderAst} from '@isograph/react';
import { Pet__PetBestFriendCard__param } from './param_type.ts';
import { Pet__PetBestFriendCard__outputType } from './output_type.ts';
import { PetBestFriendCard as resolver } from '../../../PetBestFriendCard.tsx';
import Pet__PetUpdater from '../PetUpdater/reader';

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

const artifact: ReaderArtifact<
  Pet__PetBestFriendCard__param,
  Pet__PetBestFriendCard__outputType
> = {
  kind: "ReaderArtifact",
  fieldName: "PetBestFriendCard",
  resolver: resolver as any,
  readerAst,
  variant: { kind: "Component", componentName: "Pet.PetBestFriendCard" },
};

export default artifact;
