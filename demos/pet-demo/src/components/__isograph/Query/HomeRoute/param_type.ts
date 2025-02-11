import { type Pet__PetSummaryCard__output_type } from '../../Pet/PetSummaryCard/output_type';
import type { Query__HomeRoute__parameters } from './parameters_type';

export type Query__HomeRoute__param = {
  readonly data: {
    readonly pets: ReadonlyArray<{
      readonly id: string,
      readonly PetSummaryCard: Pet__PetSummaryCard__output_type,
    }>,
  },
  readonly parameters: Query__HomeRoute__parameters,
};
