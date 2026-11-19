import type { Pet__PetName__output_type } from "./../../Pet/PetName/output_type";
import type { Query____link__output_type } from "./../__link/output_type";
export type Query__PetSummaries__param = {
  readonly data: {
    readonly pets: ReadonlyArray<{
      readonly PetName: Pet__PetName__output_type;
    }>;
  };
  readonly parameters: Record<PropertyKey, never>;
  readonly link: Query____link__output_type;
};
