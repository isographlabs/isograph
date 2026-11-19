import type { Pet____link__output_type } from './../__link/output_type';
import type { Pet__NullableCheckins__parameters } from './parameters_type';

export type Pet__NullableCheckins__param = {
  readonly data: {
    readonly checkins: ReadonlyArray<{
      readonly id: string;
    }>;
  };
  readonly parameters: Pet__NullableCheckins__parameters;
  readonly link: Pet____link__output_type;
};
