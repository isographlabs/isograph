import type {ComponentReaderArtifact, ExtractSecondParam, ReaderAst, RefetchQueryNormalizationArtifact} from '@isograph/react';
import { Pet__PetSummaryCard__param } from './param_type';
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

const artifact: ComponentReaderArtifact<
  Pet__PetSummaryCard__param,
  ExtractSecondParam<typeof resolver>
> = {
  kind: "ComponentReaderArtifact",
  componentName: "Pet.PetSummaryCard",
  resolver,
  readerAst,
};

export default artifact;
