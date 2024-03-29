import type {ReaderArtifact, ReaderAst, ExtractSecondParam} from '@isograph/react';
import { PetUpdater as resolver } from '../../../PetUpdater.tsx';
import Pet__set_pet_golden_buddy, { Pet__set_pet_golden_buddy__outputType} from '../set_pet_golden_buddy/reader';
import Pet__set_pet_tagline, { Pet__set_pet_tagline__outputType} from '../set_pet_tagline/reader';

// the type, when read out (either via useLazyReference or via graph)
export type Pet__PetUpdater__outputType = (React.FC<ExtractSecondParam<typeof resolver>>);

const readerAst: ReaderAst<Pet__PetUpdater__param> = [
  {
    kind: "MutationField",
    alias: "set_pet_golden_buddy",
    readerArtifact: Pet__set_pet_golden_buddy,
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

export type Pet__PetUpdater__param = {
  set_pet_golden_buddy: Pet__set_pet_golden_buddy__outputType,
  potential_new_best_friends: ({
    id: string,
    name: string,
  })[],
  set_pet_tagline: Pet__set_pet_tagline__outputType,
  tagline: string,
};

const artifact: ReaderArtifact<
  Pet__PetUpdater__param,
  Pet__PetUpdater__outputType
> = {
  kind: "ReaderArtifact",
  resolver: resolver as any,
  readerAst,
  variant: { kind: "Component", componentName: "Pet.PetUpdater" },
};

export default artifact;
