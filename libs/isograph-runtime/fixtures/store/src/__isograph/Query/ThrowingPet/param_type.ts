import type { Pet__ThrowingName__output_type } from './../../Pet/ThrowingName/output_type';
import type { Query____link__output_type } from './../__link/output_type';
import type { Query__ThrowingPet__parameters } from './parameters_type';

export type Query__ThrowingPet__param = {
  readonly data: {
    readonly pet: {
      readonly ThrowingName: Pet__ThrowingName__output_type;
    } | null;
  };
  readonly parameters: Query__ThrowingPet__parameters;
  readonly link: Query____link__output_type;
};
