export type Pet__PetStatsCard__param = {
  readonly data: {
    /**
     * The pet's ID
     */
    readonly id: string;
    /**
     * What you call the pet when you're very happy.
     */
    readonly nickname: (string | null);
    /**
     * Charisma? 100%
     */
    readonly stats: ({
      readonly weight: (number | null);
      readonly intelligence: (number | null);
      readonly cuteness: (number | null);
      readonly hunger: (number | null);
      readonly sociability: (number | null);
      readonly energy: (number | null);
    } | null);
  };
  readonly parameters: Record<PropertyKey, never>;
};
