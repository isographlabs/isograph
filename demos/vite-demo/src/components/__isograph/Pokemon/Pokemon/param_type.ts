
export type Pokemon__Pokemon__param = {
  readonly data: {
    /**
The dex number for a Pokémon
    */
    readonly num: number,
    /**
The species name for a Pokémon
    */
    readonly species: string,
    /**
The sprite for a Pokémon. For most Pokémon this will be the animated gif, with some exceptions that were older-gen exclusive
    */
    readonly sprite: string,
    /**
Bulbapedia page for a Pokémon
    */
    readonly bulbapediaPage: string,
  },
  readonly parameters: Record<string, never>,
};
