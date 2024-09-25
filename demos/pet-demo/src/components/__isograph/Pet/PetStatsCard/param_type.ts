import { type PetStats__refetch_pet_stats__output_type } from '../../PetStats/refetch_pet_stats/output_type';

import { type Variables } from '@isograph/react';

export type Pet__PetStatsCard__param = {
  data: {
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
      refetch_pet_stats: PetStats__refetch_pet_stats__output_type,
    } | null),
  },
  parameters: Variables,
};
