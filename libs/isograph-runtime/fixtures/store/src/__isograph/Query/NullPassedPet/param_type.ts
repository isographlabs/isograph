import type { Pet__NullableCheckins__output_type } from './../../Pet/NullableCheckins/output_type';
import type { Query____link__output_type } from './../__link/output_type';
import type { Query__NullPassedPet__parameters } from './parameters_type';

export type Query__NullPassedPet__param = {
  readonly data: {
    readonly pet: {
      readonly NullableCheckins: Pet__NullableCheckins__output_type;
    } | null;
  };
  readonly parameters: Query__NullPassedPet__parameters;
  readonly link: Query____link__output_type;
};
