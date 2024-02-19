import type {ReaderArtifact, ReaderAst} from '@isograph/react';
import { PetTaglineCard as resolver } from '../../../PetTaglineCard.tsx';

// the type, when read out (either via useLazyReference or via graph)
export type ReadOutType = (React.FC<any>);

export type ReadFromStoreType = Pet__PetTaglineCard__param;

const readerAst: ReaderAst<ReadFromStoreType> = [
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

const artifact: ReaderArtifact<ReadFromStoreType, Pet__PetTaglineCard__param, ReadOutType> = {
  kind: "ReaderArtifact",
  resolver: resolver as any,
  readerAst,
  variant: { kind: "Component", componentName: "Pet.PetTaglineCard" },
};

export default artifact;
