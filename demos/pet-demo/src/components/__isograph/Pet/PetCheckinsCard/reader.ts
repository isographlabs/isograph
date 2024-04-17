import type {ReaderArtifact, ReaderAst} from '@isograph/react';
import { Pet__PetCheckinsCard__param } from './param_type.ts';
import { Pet__PetCheckinsCard__outputType } from './output_type.ts';
import { PetCheckinsCard as resolver } from '../../../PetCheckinsCard.tsx';
import Checkin__CheckinDisplay from '../../Checkin/CheckinDisplay/reader';

const readerAst: ReaderAst<Pet__PetCheckinsCard__param> = [
  {
    kind: "Scalar",
    fieldName: "id",
    alias: null,
    arguments: null,
  },
  {
    kind: "Linked",
    fieldName: "checkins",
    alias: null,
    arguments: null,
    selections: [
      {
        kind: "Resolver",
        alias: "CheckinDisplay",
        arguments: null,
        readerArtifact: Checkin__CheckinDisplay,
        usedRefetchQueries: [0, ],
      },
      {
        kind: "Scalar",
        fieldName: "id",
        alias: null,
        arguments: null,
      },
    ],
  },
];

const artifact: ReaderArtifact<
  Pet__PetCheckinsCard__param,
  Pet__PetCheckinsCard__outputType
> = {
  kind: "ReaderArtifact",
  fieldName: "PetCheckinsCard",
  resolver: resolver as any,
  readerAst,
  variant: { kind: "Component", componentName: "Pet.PetCheckinsCard" },
};

export default artifact;
