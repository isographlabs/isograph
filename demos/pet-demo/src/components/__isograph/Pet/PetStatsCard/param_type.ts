import {PetStats__refetch_pet_stats__outputType} from '../../PetStats/refetch_pet_stats/output_type';

export type Pet__PetStatsCard__param = {
  id: string,
  nickname: (string | null),
  age: number,
  stats: ({
    weight: (number | null),
    intelligence: (number | null),
    cuteness: (number | null),
    hunger: (number | null),
    sociability: (number | null),
    energy: (number | null),
    refetch_pet_stats: PetStats__refetch_pet_stats__outputType,
  } | null),
};
