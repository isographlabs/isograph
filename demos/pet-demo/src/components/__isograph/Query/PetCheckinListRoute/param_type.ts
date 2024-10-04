import { type Pet__FirstCheckinMakeSuperButton__output_type } from '../../Pet/FirstCheckinMakeSuperButton/output_type';
import { type Pet__PetCheckinsCardList__output_type } from '../../Pet/PetCheckinsCardList/output_type';

import { type LoadableField } from '@isograph/react';
import { type Variables } from '@isograph/react';

export type Query__PetCheckinListRoute__param = {
  readonly data: {
    readonly pet: ({
      readonly FirstCheckinMakeSuperButton: Pet__FirstCheckinMakeSuperButton__output_type,
      readonly name: string,
      readonly PetCheckinsCardList: LoadableField<{readonly skip?: number | null | void, readonly limit?: number | null | void}, Pet__PetCheckinsCardList__output_type>,
    } | null),
  },
  readonly parameters: Variables,
};
