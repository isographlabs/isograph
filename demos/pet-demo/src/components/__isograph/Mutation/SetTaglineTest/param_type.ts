import { type Mutation__TestLazyReference__output_type } from '../../Mutation/TestLazyReference/output_type';
import { type LoadableField, type ExtractParameters } from '@isograph/react';
import { type Mutation__TestLazyReference__param } from '../../Mutation/TestLazyReference/param_type';
import type { Mutation__SetTaglineTest__parameters } from './parameters_type';

export type Mutation__SetTaglineTest__param = {
  readonly data: {
    readonly set_pet_tagline: {
      readonly pet: {
        readonly id: string,
      },
    },
    readonly TestLazyReference: LoadableField<
      Mutation__TestLazyReference__param,
      Mutation__TestLazyReference__output_type
    >,
  },
  readonly parameters: Mutation__SetTaglineTest__parameters,
};
