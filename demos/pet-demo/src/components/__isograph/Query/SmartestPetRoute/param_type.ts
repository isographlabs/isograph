import { type Pet__Avatar__output_type } from '../../Pet/Avatar/output_type';
import { type Pet__fullName__output_type } from '../../Pet/fullName/output_type';
import { type LoadableField, type ExtractParameters } from '@isograph/react';
import { type Pet__firstCheckin__param } from '../../Pet/firstCheckin/param_type';
import { type Query__smartestPet__param } from '../../Query/smartestPet/param_type';

export type Query__SmartestPetRoute__param = {
  readonly data: {
    readonly smartestPet: (LoadableField<Query__smartestPet__param, {
      readonly id: string,
      readonly fullName: Pet__fullName__output_type,
      readonly Avatar: Pet__Avatar__output_type,
      readonly stats: ({
        readonly intelligence: (number | null),
      } | null),
      readonly picture: string,
      readonly firstCheckin: (LoadableField<Pet__firstCheckin__param, {
        readonly location: string,
      }> | null),
    }> | null),
  },
  readonly parameters: Record<PropertyKey, never>,
};
