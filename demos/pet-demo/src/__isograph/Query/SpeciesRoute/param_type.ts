import type { Pet__fullName__output_type } from "./../../Pet/fullName/output_type";
import type { Query__SpeciesRoute__parameters } from "./parameters_type";
export type Query__SpeciesRoute__param = {
  readonly data: {
    /**
     * Every pet of one species.
     */
    readonly petsBySpecies: ReadonlyArray<{
      /**
       * The pet's ID
       */
      readonly id: string;
      /**
       * The pet's first and last name, for example Makayla Balicki.
       */
      readonly fullName: Pet__fullName__output_type;
      /**
       * It's probably something from David Foster Wallace or Dostoevsky, keep up.
       */
      readonly favorite_phrase: (string | null);
    }>;
    /**
     * Who are you? Find out here!
     */
    readonly viewer: {
      readonly id: string;
    };
    /**
     * Fetch any object that has an id
     */
    readonly node: ({
      readonly id: string;
    } | null);
  };
  readonly parameters: Query__SpeciesRoute__parameters;
};
