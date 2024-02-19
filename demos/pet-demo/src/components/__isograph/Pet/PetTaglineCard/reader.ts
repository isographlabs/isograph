import type {ReaderArtifact, ReaderAst} from '@isograph/react';
import { PetTaglineCard as resolver } from '../../../PetTaglineCard.tsx';

// the type, when read out (either via useLazyReference or via graph)
export type Pet__PetTaglineCard__outputType = (React.FC<any>);

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

export type Pet__PetTaglineCard__param = { data:
{
  id: string,
  tagline: string,
},
[index: string]: any };

const artifact: ReaderArtifact<
  Pet__PetTaglineCard__param,
  Pet__PetTaglineCard__param,
  Pet__PetTaglineCard__outputType
> = {
  kind: "ReaderArtifact",
  resolver: resolver as any,
  readerAst,
  variant: { kind: "Component", componentName: "Pet.PetTaglineCard" },
};

export default artifact;
