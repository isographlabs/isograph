import type { Pet__PetBestFriendCard__output_type } from "./../../Pet/PetBestFriendCard/output_type";
import type { Pet__PetCheckinsCard__output_type } from "./../../Pet/PetCheckinsCard/output_type";
import type { Pet__PetStatsCard__output_type } from "./../../Pet/PetStatsCard/output_type";
import type { Pet__fullName__output_type } from "./../../Pet/fullName/output_type";
import type { Query____link__output_type } from "./../__link/output_type";
import type { Query__PetDetailRoute__parameters } from "./parameters_type";
export type Query__PetDetailRoute__param = {
  readonly data: {
    /**
     * Fetch a pet by id
     */
    readonly pet: ({
      /**
       * The pet's ID
       */
      readonly id: string;
      /**
       * The pet's first and last name, for example Makayla Balicki.
       */
      readonly fullName: Pet__fullName__output_type;
      readonly PetStatsCard: Pet__PetStatsCard__output_type;
      readonly PetCheckinsCard: Pet__PetCheckinsCard__output_type;
      readonly PetBestFriendCard: Pet__PetBestFriendCard__output_type;
    } | null);
  };
  readonly parameters: Query__PetDetailRoute__parameters;
  readonly link: Query____link__output_type;
};
