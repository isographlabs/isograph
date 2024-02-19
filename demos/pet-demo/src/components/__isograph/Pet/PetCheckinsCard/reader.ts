import type {ReaderArtifact, ReaderAst} from '@isograph/react';
import { PetCheckinsCard as resolver } from '../../../PetCheckinsCard.tsx';

// the type, when read out (either via useLazyReference or via graph)
export type Pet__PetCheckinsCard__outputType = (React.FC<any>);

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

export type Pet__PetCheckinsCard__param = { data:
{
  id: string,
  checkins: ({
    id: string,
    location: string,
    time: string,
  })[],
},
[index: string]: any };

const artifact: ReaderArtifact<
  Pet__PetCheckinsCard__param,
  Pet__PetCheckinsCard__param,
  Pet__PetCheckinsCard__outputType
> = {
  kind: "ReaderArtifact",
  resolver: resolver as any,
  readerAst,
  variant: { kind: "Component", componentName: "Pet.PetCheckinsCard" },
};

export default artifact;
