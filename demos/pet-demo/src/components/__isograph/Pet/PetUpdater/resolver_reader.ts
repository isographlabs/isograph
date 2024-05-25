import type {ComponentReaderArtifact, ExtractSecondParam, ReaderAst, RefetchQueryNormalizationArtifact} from '@isograph/react';
import { Pet__PetUpdater__param } from './param_type';
import { PetUpdater as resolver } from '../../../PetUpdater.tsx';
import Pet____refetch__resolver_reader from '../__refetch/refetch_reader';
import Pet__set_best_friend__resolver_reader from '../set_best_friend/refetch_reader';
import Pet__set_pet_tagline__resolver_reader from '../set_pet_tagline/refetch_reader';

const readerAst: ReaderAst<Pet__PetUpdater__param> = [
  {
    kind: "MutationField",
    alias: "set_best_friend",
    // @ts-ignore
    readerArtifact: Pet__set_best_friend__resolver_reader,
    refetchQuery: 1,
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
    // @ts-ignore
    readerArtifact: Pet__set_pet_tagline__resolver_reader,
    refetchQuery: 2,
  },
  {
    kind: "Scalar",
    fieldName: "tagline",
    alias: null,
    arguments: null,
  },
  {
    kind: "RefetchField",
    alias: "__refetch",
    readerArtifact: Pet____refetch__resolver_reader,
    refetchQuery: 0,
  },
];

const artifact: ComponentReaderArtifact<
  Pet__PetUpdater__param,
  ExtractSecondParam<typeof resolver>
> = {
  kind: "ComponentReaderArtifact",
  componentName: "Pet.PetUpdater",
  resolver,
  readerAst,
};

export default artifact;
