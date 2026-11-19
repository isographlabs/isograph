import type { Pet__fullName__output_type } from "./../fullName/output_type";
export type Pet__PetSummaryCard__param = {
  readonly data: {
    /**
     * The pet's ID
     */
    readonly id: string;
    /**
     * The pet's first and last name, for example Makayla Balicki.
     */
    readonly fullName: Pet__fullName__output_type;
    /**
     * What you call the pet when you're very happy.
     */
    readonly nickname: (string | null);
    /**
     * It's just a number.
     */
    readonly age: number;
    readonly picture: unknown;
    /**
     * If your pet was a superhero.
     */
    readonly tagline: string;
    /**
     * What kind of animal the pet is.
     */
    readonly species: string;
  };
  readonly parameters: Record<PropertyKey, never>;
};
