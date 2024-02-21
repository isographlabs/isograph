import type {ReaderArtifact, ReaderAst} from '@isograph/react';
import { PetStatsCard as resolver } from '../../../PetStatsCard.tsx';
import Pet____refetch, { Pet____refetch__outputType} from '../__refetch/reader';

// the type, when read out (either via useLazyReference or via graph)
export type Pet__PetStatsCard__outputType = (React.FC<any>);

const readerAst: ReaderAst<Pet__PetStatsCard__param> = [
  {
    kind: "Scalar",
    fieldName: "id",
    alias: null,
    arguments: null,
  },
  {
    kind: "Scalar",
    fieldName: "nickname",
    alias: null,
    arguments: null,
  },
  {
    kind: "RefetchField",
    alias: "__refetch",
    readerArtifact: Pet____refetch,
    refetchQuery: 0,
  },
  {
    kind: "Scalar",
    fieldName: "age",
    alias: null,
    arguments: null,
  },
  {
    kind: "Linked",
    fieldName: "stats",
    alias: null,
    arguments: null,
    selections: [
      {
        kind: "Scalar",
        fieldName: "weight",
        alias: null,
        arguments: null,
      },
      {
        kind: "Scalar",
        fieldName: "intelligence",
        alias: null,
        arguments: null,
      },
      {
        kind: "Scalar",
        fieldName: "cuteness",
        alias: null,
        arguments: null,
      },
      {
        kind: "Scalar",
        fieldName: "hunger",
        alias: null,
        arguments: null,
      },
      {
        kind: "Scalar",
        fieldName: "sociability",
        alias: null,
        arguments: null,
      },
      {
        kind: "Scalar",
        fieldName: "energy",
        alias: null,
        arguments: null,
      },
    ],
  },
];

export type Pet__PetStatsCard__param = { data:
{
  id: string,
  nickname: (string | null),
  __refetch: Pet____refetch__outputType,
  age: number,
  stats: {
    weight: number,
    intelligence: number,
    cuteness: number,
    hunger: number,
    sociability: number,
    energy: number,
  },
},
[index: string]: any };

const artifact: ReaderArtifact<
  Pet__PetStatsCard__param,
  Pet__PetStatsCard__param,
  Pet__PetStatsCard__outputType
> = {
  kind: "ReaderArtifact",
  resolver: resolver as any,
  readerAst,
  variant: { kind: "Component", componentName: "Pet.PetStatsCard" },
};

export default artifact;
