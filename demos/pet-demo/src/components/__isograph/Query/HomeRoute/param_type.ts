import { type Pet__PetSummaryCard__output_type } from '../../Pet/PetSummaryCard/output_type';

export type Query__HomeRoute__param = {
  pets: ({
    id: string,
    PetSummaryCard: Pet__PetSummaryCard__output_type,
  })[],
};
