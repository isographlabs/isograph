import type {ComponentReaderArtifact, ExtractSecondParam, ReaderAst } from '@isograph/react';
import { Pet__PetStatsCard__param } from './param_type';
import { PetStatsCard as resolver } from '../../../Pet/PetStatsCard';
import PetStats__refetch_pet_stats__refetch_reader from '../../PetStats/refetch_pet_stats/refetch_reader';

const readerAst: ReaderAst<Pet__PetStatsCard__param> = [
  {
    kind: "Scalar",
    isFallible: false,
    fieldName: "id",
    alias: null,
    arguments: null,
    isUpdatable: false,
  },
  {
    kind: "Scalar",
    isFallible: true,
    fieldName: "nickname",
    alias: null,
    arguments: null,
    isUpdatable: false,
  },
  {
    kind: "Scalar",
    isFallible: false,
    fieldName: "age",
    alias: null,
    arguments: null,
    isUpdatable: false,
  },
  {
    kind: "Linked",
    isFallible: true,
    fieldName: "stats",
    alias: null,
    arguments: null,
    condition: null,
    isUpdatable: false,
    refetchQueryIndex: null,
    selections: [
      {
        kind: "Scalar",
        isFallible: true,
        fieldName: "weight",
        alias: null,
        arguments: null,
        isUpdatable: false,
      },
      {
        kind: "Scalar",
        isFallible: true,
        fieldName: "intelligence",
        alias: null,
        arguments: null,
        isUpdatable: false,
      },
      {
        kind: "Scalar",
        isFallible: true,
        fieldName: "cuteness",
        alias: null,
        arguments: null,
        isUpdatable: false,
      },
      {
        kind: "Scalar",
        isFallible: true,
        fieldName: "hunger",
        alias: null,
        arguments: null,
        isUpdatable: false,
      },
      {
        kind: "Scalar",
        isFallible: true,
        fieldName: "sociability",
        alias: null,
        arguments: null,
        isUpdatable: false,
      },
      {
        kind: "Scalar",
        isFallible: true,
        fieldName: "energy",
        alias: null,
        arguments: null,
        isUpdatable: false,
      },
      {
        kind: "ImperativelyLoadedField",
        alias: "refetch_pet_stats",
        refetchReaderArtifact: PetStats__refetch_pet_stats__refetch_reader,
        refetchQueryIndex: 0,
        name: "refetch_pet_stats",
      },
    ],
  },
];

const artifact = (): ComponentReaderArtifact<
  Pet__PetStatsCard__param,
  ExtractSecondParam<typeof resolver>
> => ({
  kind: "ComponentReaderArtifact",
  fieldName: "PetStatsCard",
  resolver,
  readerAst,
  hasUpdatable: false,
});

export default artifact;
