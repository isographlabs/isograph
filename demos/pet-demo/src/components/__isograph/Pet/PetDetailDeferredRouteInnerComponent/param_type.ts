import { type Pet__PetCheckinsCard__output_type } from '../../Pet/PetCheckinsCard/output_type';

import { type LoadableField } from '@isograph/react';
import { type Variables } from '@isograph/react';

export type Pet__PetDetailDeferredRouteInnerComponent__param = {
  data: {
    name: string,
    PetCheckinsCard: LoadableField<{skip?: number | null | void, limit?: number | null | void}, Pet__PetCheckinsCard__output_type>,
  },
  parameters: Variables,
};
