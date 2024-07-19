import { type Pet__PetCheckinsCard__output_type } from '../../Pet/PetCheckinsCard/output_type';

import { type LoadableField } from '@isograph/react';
export type Query__PetDetailDeferredRoute__param = {
  pet: ({
    name: string,
    PetCheckinsCard: LoadableField<Pet__PetCheckinsCard__output_type>,
  } | null),
};
