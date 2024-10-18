import type {ComponentReaderArtifact, ExtractSecondParam, ReaderAst } from '@isograph/react';
import { Pet__PetStatsCard__param } from './param_type';
import { PetStatsCard as resolver } from '../../../PetStatsCard';
import PetStats__refetch_pet_stats__refetch_reader from '../../PetStats/refetch_pet_stats/refetch_reader';

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
      {
        kind: "ImperativelyLoadedField",
        alias: "refetch_pet_stats",
        refetchReaderArtifact: PetStats__refetch_pet_stats__refetch_reader,
        refetchQuery: 0,
        name: "refetch_pet_stats",
      },
    ],
  },
];

const artifact: ComponentReaderArtifact<
  Pet__PetStatsCard__param,
  ExtractSecondParam<typeof resolver>
> = {
  kind: "ComponentReaderArtifact",
  componentName: "Pet.PetStatsCard",
  resolver,
  readerAst,
};

export default artifact;
