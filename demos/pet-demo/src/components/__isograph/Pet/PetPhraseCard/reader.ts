import type {ReaderArtifact, ReaderAst} from '@isograph/react';
import { Pet__PetPhraseCard__param } from './param_type.ts';
import { Pet__PetPhraseCard__outputType } from './output_type.ts';
import { PetPhraseCard as resolver } from '../../../PetPhraseCard.tsx';

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

const artifact: ReaderArtifact<
  Pet__PetPhraseCard__param,
  Pet__PetPhraseCard__outputType
> = {
  kind: "ReaderArtifact",
  fieldName: "PetPhraseCard",
  resolver: resolver as any,
  readerAst,
  variant: { kind: "Component", componentName: "Pet.PetPhraseCard" },
};

export default artifact;
