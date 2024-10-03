import { type Pet__PetCheckinsCard__output_type } from '../../Pet/PetCheckinsCard/output_type';

import { type LoadableField } from '@isograph/react';
import { type Variables } from '@isograph/react';

export type Pet__PetDetailDeferredRouteInnerComponent__param = {
  readonly data: {
    readonly name: string,
    readonly PetCheckinsCard: LoadableField<{readonly skip?: number | null | void, readonly limit?: number | null | void}, Pet__PetCheckinsCard__output_type>,
  },
  readonly parameters: Variables,
};
