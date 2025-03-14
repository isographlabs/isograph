import { type Pet__FirstCheckinMakeSuperButton__output_type } from '../../Pet/FirstCheckinMakeSuperButton/output_type';
import { type Pet__PetCheckinsCardList__output_type } from '../../Pet/PetCheckinsCardList/output_type';
import { type LoadableField, type ExtractParameters } from '@isograph/react';
import { type Pet__PetCheckinsCardList__param } from '../../Pet/PetCheckinsCardList/param_type';
import type { Query__PetCheckinListRoute__parameters } from './parameters_type';

export type Query__PetCheckinListRoute__param = {
  readonly data: {
    readonly pet: ({
      readonly FirstCheckinMakeSuperButton: Pet__FirstCheckinMakeSuperButton__output_type,
      readonly name: string,
      readonly PetCheckinsCardList: LoadableField<
        Pet__PetCheckinsCardList__param,
        Pet__PetCheckinsCardList__output_type
      >,
    } | null),
  },
  readonly parameters: Query__PetCheckinListRoute__parameters,
};
