import type {ReaderArtifact, ReaderAst} from '@isograph/react';
import { PetSummaryCard as resolver } from '../../../PetSummaryCard.tsx';

// the type, when read out (either via useLazyReference or via graph)
export type Pet__PetSummaryCard__outputType = (React.FC<any>);

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
];

export type Pet__PetSummaryCard__param = { data:
{
  id: string,
  name: string,
  picture: string,
  tagline: string,
},
[index: string]: any };

const artifact: ReaderArtifact<
  Pet__PetSummaryCard__param,
  Pet__PetSummaryCard__param,
  Pet__PetSummaryCard__outputType
> = {
  kind: "ReaderArtifact",
  resolver: resolver as any,
  readerAst,
  variant: { kind: "Component", componentName: "Pet.PetSummaryCard" },
};

export default artifact;
