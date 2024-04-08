import type {ReaderArtifact, ReaderAst} from '@isograph/react';
import { Pet__PetTaglineCard__param } from './param_type.ts';
import { Pet__PetTaglineCard__outputType } from './output_type.ts';
import { PetTaglineCard as resolver } from '../../../PetTaglineCard.tsx';

const readerAst: ReaderAst<Pet__PetTaglineCard__param> = [
  {
    kind: "Scalar",
    fieldName: "id",
    alias: null,
    arguments: null,
  },
  {
    kind: "Scalar",
    fieldName: "tagline",
    alias: null,
    arguments: null,
  },
];

const artifact: ReaderArtifact<
  Pet__PetTaglineCard__param,
  Pet__PetTaglineCard__outputType
> = {
  kind: "ReaderArtifact",
  resolver: resolver as any,
  readerAst,
  variant: { kind: "Component", componentName: "Pet.PetTaglineCard" },
};

export default artifact;
