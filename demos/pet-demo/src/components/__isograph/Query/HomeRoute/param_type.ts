import { type Pet__PetSummaryCard__output_type } from '../../Pet/PetSummaryCard/output_type';

export type Query__HomeRoute__param = {
  readonly data: {
    /**
All the pets! What more could you ask for?
    */
    readonly pets: ReadonlyArray<{
      /**
The pet's ID
      */
      readonly id: string,
      readonly PetSummaryCard: Pet__PetSummaryCard__output_type,
    }>,
  },
  readonly parameters: Record<PropertyKey, never>,
};
