import { type Pet__PetSummaryCard__output_type } from '../../Pet/PetSummaryCard/output_type';

import { type Variables } from '@isograph/react';

export type Query__HomeRoute__param = {
  data: {
    pets: ({
      id: string,
      PetSummaryCard: Pet__PetSummaryCard__output_type,
    })[],
  },
  parameters: Variables,
};
