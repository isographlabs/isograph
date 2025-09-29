import type {ComponentReaderArtifact, ExtractSecondParam, ReaderAst } from '@isograph/react';
import { Pet__PetTaglineTestCard__param } from './param_type';
import { SetTaglineTest as resolver } from '../../../Pet/PetTaglineCard2';

const readerAst: ReaderAst<Pet__PetTaglineTestCard__param> = [
  {
    kind: "Scalar",
    fieldName: "id",
    alias: null,
    arguments: null,
    isUpdatable: false,
  },
];

const artifact: ComponentReaderArtifact<
  Pet__PetTaglineTestCard__param,
  ExtractSecondParam<typeof resolver>
> = {
  kind: "ComponentReaderArtifact",
  fieldName: "Pet.PetTaglineTestCard",
  resolver,
  readerAst,
  hasUpdatable: false,
};

export default artifact;
