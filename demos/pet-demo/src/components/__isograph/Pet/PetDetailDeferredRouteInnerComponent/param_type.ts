import { type Pet__PetCheckinsCard__output_type } from '../../Pet/PetCheckinsCard/output_type';
import { type LoadableField } from '@isograph/react';
import { type Pet__PetCheckinsCard__param } from '../../Pet/PetCheckinsCard/param_type';

export type Pet__PetDetailDeferredRouteInnerComponent__param = {
  readonly data: {
    readonly name: string,
    readonly PetCheckinsCard: LoadableField<
      Pet__PetCheckinsCard__param,
      Pet__PetCheckinsCard__output_type
    >,
  },
  readonly parameters: Record<string, never>,
};
