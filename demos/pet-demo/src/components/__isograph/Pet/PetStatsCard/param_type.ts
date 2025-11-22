import { type PetStats__refetch_pet_stats__output_type } from '../../PetStats/refetch_pet_stats/output_type';
import type { Pet__PetStatsCard__parameters } from './parameters_type';

export type Pet__PetStatsCard__param = {
  readonly data: {
    /**
The pet's ID
    */
    readonly id: string,
    /**
What you call the pet when you're very happy.
    */
    readonly nickname: (string | null),
    /**
It's just a number.
    */
    readonly age: number,
    /**
Charisma? 100%
    */
    readonly stats: ({
      readonly weight: (number | null),
      readonly intelligence: (number | null),
      readonly cuteness: (number | null),
      readonly hunger: (number | null),
      readonly sociability: (number | null),
      readonly energy: (number | null),
      /**
Fetch a pet by id
      */
      readonly refetch_pet_stats: PetStats__refetch_pet_stats__output_type,
    } | null),
  },
  readonly parameters: Pet__PetStatsCard__parameters,
};
