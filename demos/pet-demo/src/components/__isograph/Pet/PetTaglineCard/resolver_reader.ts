import type {ComponentReaderArtifact, ExtractSecondParam, ReaderAst } from '@isograph/react';
import { Pet__PetTaglineCard__param } from './param_type';
import { PetTaglineCard as resolver } from '../../../PetTaglineCard';

const readerAst: ReaderAst<Pet__PetTaglineCard__param> = [
  {
    kind: "Scalar",
    fieldName: "id",
    alias: null,
    arguments: null,
    isUpdatable: false,
  },
  {
    kind: "Scalar",
    fieldName: "tagline",
    alias: null,
    arguments: null,
    isUpdatable: false,
  },
];

const artifact: ComponentReaderArtifact<
  Pet__PetTaglineCard__param,
  ExtractSecondParam<typeof resolver>
> = {
  kind: "ComponentReaderArtifact",
  fieldName: "Pet.PetTaglineCard",
  resolver,
  readerAst,
  hasUpdatable: false,
};

export default artifact;
