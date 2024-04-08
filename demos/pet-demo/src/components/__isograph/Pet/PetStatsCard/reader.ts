import type {ReaderArtifact, ReaderAst} from '@isograph/react';
import { Pet__PetStatsCard__param } from './param_type.ts';
import { Pet__PetStatsCard__outputType } from './output_type.ts';
import { PetStatsCard as resolver } from '../../../PetStatsCard.tsx';
import Pet____refetch from '../__refetch/reader';

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

const artifact: ReaderArtifact<
  Pet__PetStatsCard__param,
  Pet__PetStatsCard__outputType
> = {
  kind: "ReaderArtifact",
  resolver: resolver as any,
  readerAst,
  variant: { kind: "Component", componentName: "Pet.PetStatsCard" },
};

export default artifact;
