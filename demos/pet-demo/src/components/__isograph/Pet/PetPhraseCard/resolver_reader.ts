import type {ComponentReaderArtifact, ExtractSecondParam, ReaderAst } from '@isograph/react';
import { Pet__PetPhraseCard__param } from './param_type';
import { PetPhraseCard as resolver } from '../../../PetPhraseCard';

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
