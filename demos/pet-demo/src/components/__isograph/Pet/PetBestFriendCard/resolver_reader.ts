import type {ComponentReaderArtifact, ExtractSecondParam, ReaderAst } from '@isograph/react';
import { Pet__PetBestFriendCard__param } from './param_type';
import { PetBestFriendCard as resolver } from '../../../Pet/PetBestFriendCard';
import Pet__Avatar__resolver_reader from '../../Pet/Avatar/resolver_reader';
import Pet__PetUpdater__resolver_reader from '../../Pet/PetUpdater/resolver_reader';

const readerAst: ReaderAst<Pet__PetBestFriendCard__param> = [
  {
    kind: "Scalar",
    fieldName: "id",
    alias: null,
    arguments: null,
    isUpdatable: false,
  },
  {
    kind: "Resolver",
    alias: "PetUpdater",
    arguments: null,
    readerArtifact: Pet__PetUpdater__resolver_reader,
    usedRefetchQueries: [0, 1, 2, ],
  },
  {
    kind: "Linked",
    fieldName: "best_friend_relationship",
    alias: null,
    arguments: null,
    condition: null,
    isUpdatable: false,
    selections: [
      {
        kind: "Scalar",
        fieldName: "picture_together",
        alias: null,
        arguments: null,
        isUpdatable: false,
      },
      {
        kind: "Linked",
        fieldName: "best_friend",
        alias: null,
        arguments: null,
        condition: null,
        isUpdatable: false,
        selections: [
          {
            kind: "Scalar",
            fieldName: "id",
            alias: null,
            arguments: null,
            isUpdatable: false,
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
            alias: "Avatar",
            arguments: null,
            readerArtifact: Pet__Avatar__resolver_reader,
            usedRefetchQueries: [],
          },
        ],
        refetchQueryIndex: null,
      },
    ],
    refetchQueryIndex: null,
  },
];

const artifact: ComponentReaderArtifact<
  Pet__PetBestFriendCard__param,
  ExtractSecondParam<typeof resolver>
> = {
  kind: "ComponentReaderArtifact",
  fieldName: "Pet.PetBestFriendCard",
  resolver,
  readerAst,
  hasUpdatable: false,
};

export default artifact;
