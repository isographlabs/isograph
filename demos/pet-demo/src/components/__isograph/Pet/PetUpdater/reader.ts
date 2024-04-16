import type {ReaderArtifact, ReaderAst} from '@isograph/react';
import { Pet__PetUpdater__param } from './param_type.ts';
import { Pet__PetUpdater__outputType } from './output_type.ts';
import { PetUpdater as resolver } from '../../../PetUpdater.tsx';
import Pet__set_best_friend from '../set_best_friend/reader';
import Pet__set_pet_tagline from '../set_pet_tagline/reader';

const readerAst: ReaderAst<Pet__PetUpdater__param> = [
  {
    kind: "MutationField",
    alias: "set_best_friend",
    readerArtifact: Pet__set_best_friend,
    refetchQuery: 0,
  },
  {
    kind: "Linked",
    fieldName: "potential_new_best_friends",
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
    ],
  },
  {
    kind: "MutationField",
    alias: "set_pet_tagline",
    readerArtifact: Pet__set_pet_tagline,
    refetchQuery: 1,
  },
  {
    kind: "Scalar",
    fieldName: "tagline",
    alias: null,
    arguments: null,
  },
];

const artifact: ReaderArtifact<
  Pet__PetUpdater__param,
  Pet__PetUpdater__outputType
> = {
  kind: "ReaderArtifact",
  fieldName: "PetUpdater",
  resolver: resolver as any,
  readerAst,
  variant: { kind: "Component", componentName: "Pet.PetUpdater" },
};

export default artifact;
