import type {ComponentReaderArtifact, ExtractSecondParam, ReaderAst } from '@isograph/react';
import { Pet__PetUpdater__param } from './param_type';
import { PetUpdater as resolver } from '../../../Pet/PetUpdater';
import Pet____refetch__refetch_reader from '../../Pet/__refetch/refetch_reader';
import Pet__set_best_friend__refetch_reader from '../../Pet/set_best_friend/refetch_reader';
import Pet__set_pet_tagline__refetch_reader from '../../Pet/set_pet_tagline/refetch_reader';

const readerAst: ReaderAst<Pet__PetUpdater__param> = [
  {
    kind: "ImperativelyLoadedField",
    alias: "set_best_friend",
    refetchReaderArtifact: Pet__set_best_friend__refetch_reader,
    refetchQueryIndex: 1,
    name: "set_best_friend",
  },
  {
    kind: "Linked",
    fieldName: "potential_new_best_friends",
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
    ],
    refetchQueryIndex: null,
  },
  {
    kind: "ImperativelyLoadedField",
    alias: "set_pet_tagline",
    refetchReaderArtifact: Pet__set_pet_tagline__refetch_reader,
    refetchQueryIndex: 2,
    name: "set_pet_tagline",
  },
  {
    kind: "Scalar",
    fieldName: "tagline",
    alias: null,
    arguments: null,
    isUpdatable: true,
  },
  {
    kind: "ImperativelyLoadedField",
    alias: "__refetch",
    refetchReaderArtifact: Pet____refetch__refetch_reader,
    refetchQueryIndex: 0,
    name: "__refetch",
  },
];

const artifact: ComponentReaderArtifact<
  Pet__PetUpdater__param,
  ExtractSecondParam<typeof resolver>
> = {
  kind: "ComponentReaderArtifact",
  fieldName: "Pet.PetUpdater",
  resolver,
  readerAst,
  hasUpdatable: true,
};

export default artifact;
