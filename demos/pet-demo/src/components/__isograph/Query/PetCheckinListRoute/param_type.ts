import { type Pet__PetCheckinsCardList__output_type } from '../../Pet/PetCheckinsCardList/output_type';

import { type LoadableField } from '@isograph/react';
import { type Variables } from '@isograph/react';

export type Query__PetCheckinListRoute__param = {
  data: {
    pet: ({
      name: string,
      PetCheckinsCardList: LoadableField<{skip?: number | null | void, limit?: number | null | void}, Pet__PetCheckinsCardList__output_type>,
    } | null),
  },
  parameters: Variables,
};
