import type {ReaderArtifact, ReaderAst} from '@isograph/react';
import { Pet__PetSummaryCard__param } from './param_type.ts';
import { Pet__PetSummaryCard__outputType } from './output_type.ts';
import { PetSummaryCard as resolver } from '../../../PetSummaryCard.tsx';
import Pet__FavoritePhraseLoader from '../FavoritePhraseLoader/reader';

const readerAst: ReaderAst<Pet__PetSummaryCard__param> = [
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
  {
    kind: "Scalar",
    fieldName: "tagline",
    alias: null,
    arguments: null,
  },
  {
    kind: "Resolver",
    alias: "FavoritePhraseLoader",
    arguments: null,
    readerArtifact: Pet__FavoritePhraseLoader,
    usedRefetchQueries: [],
  },
];

const artifact: ReaderArtifact<
  Pet__PetSummaryCard__param,
  Pet__PetSummaryCard__outputType
> = {
  kind: "ReaderArtifact",
  fieldName: "PetSummaryCard",
  resolver: resolver as any,
  readerAst,
  variant: { kind: "Component", componentName: "Pet.PetSummaryCard" },
};

export default artifact;
