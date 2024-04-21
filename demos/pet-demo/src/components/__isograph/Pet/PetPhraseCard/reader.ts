import type {ComponentReaderArtifact, ExtractSecondParam, ReaderAst, RefetchQueryNormalizationArtifact} from '@isograph/react';
import { Pet__PetPhraseCard__param } from './param_type';
import { PetPhraseCard as resolver } from '../../../PetPhraseCard.tsx';
import Pet__set_pet_tagline from '../set_pet_tagline/reader';

const readerAst: ReaderAst<Pet__PetPhraseCard__param> = [
  {
    kind: "Scalar",
    fieldName: "id",
    alias: null,
    arguments: null,
  },
  {
    kind: "Scalar",
    fieldName: "favorite_phrase",
    alias: null,
    arguments: null,
  },
  {
    kind: "MutationField",
    alias: "set_pet_tagline",
    readerArtifact: Pet__set_pet_tagline,
    refetchQuery: 0,
  },
];

const artifact: ComponentReaderArtifact<
  Pet__PetPhraseCard__param,
  ExtractSecondParam<typeof resolver>
> = {
  kind: "ComponentReaderArtifact",
  componentName: "Pet.PetPhraseCard",
  resolver,
  readerAst,
};

export default artifact;
