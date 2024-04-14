import type {ReaderArtifact, ReaderAst, ExtractSecondParam} from '@isograph/react';
import { PetCheckinsCard as resolver } from '../../../PetCheckinsCard.tsx';
import Checkin__CheckinDisplay, { Checkin__CheckinDisplay__outputType} from '../../Checkin/CheckinDisplay/reader';

// the type, when read out (either via useLazyReference or via graph)
export type Pet__PetCheckinsCard__outputType = (React.FC<ExtractSecondParam<typeof resolver>>);

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
      {
        kind: "Scalar",
        fieldName: "location",
        alias: null,
        arguments: null,
      },
      {
        kind: "Scalar",
        fieldName: "time",
        alias: null,
        arguments: null,
      },
    ],
  },
];

export type Pet__PetCheckinsCard__param = {
  id: string,
  checkins: ({
    CheckinDisplay: Checkin__CheckinDisplay__outputType,
    id: string,
    location: string,
    time: string,
  })[],
};

const artifact: ReaderArtifact<
  Pet__PetCheckinsCard__param,
  Pet__PetCheckinsCard__outputType
> = {
  kind: "ReaderArtifact",
  resolver: resolver as any,
  readerAst,
  variant: { kind: "Component", componentName: "Pet.PetCheckinsCard" },
};

export default artifact;
