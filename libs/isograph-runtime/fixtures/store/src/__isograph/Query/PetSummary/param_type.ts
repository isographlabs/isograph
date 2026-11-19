import type { Pet__PetName__output_type } from './../../Pet/PetName/output_type';
import type { Pet__RecentCheckins__output_type } from './../../Pet/RecentCheckins/output_type';
import type { Query____link__output_type } from './../__link/output_type';
import type { Query__PetSummary__parameters } from './parameters_type';

export type Query__PetSummary__param = {
  readonly data: {
    readonly pet: {
      readonly id: string;
      readonly PetName: Pet__PetName__output_type;
      readonly RecentCheckins: Pet__RecentCheckins__output_type;
    } | null;
  };
  readonly parameters: Query__PetSummary__parameters;
  readonly link: Query____link__output_type;
};
