import { type PetStats__refetch_pet_stats__output_type } from '../../PetStats/refetch_pet_stats/output_type';
import type { Pet__PetStatsCard__parameters } from './parameters_type';

export type Pet__PetStatsCard__param = {
  readonly data: {
    readonly id: string,
    readonly nickname: (string | null),
    readonly age: number,
    readonly stats: ({
      readonly weight: (number | null),
      readonly intelligence: (number | null),
      readonly cuteness: (number | null),
      readonly hunger: (number | null),
      readonly sociability: (number | null),
      readonly energy: (number | null),
      readonly refetch_pet_stats: PetStats__refetch_pet_stats__output_type,
    } | null),
  },
  readonly parameters: Pet__PetStatsCard__parameters,
};
