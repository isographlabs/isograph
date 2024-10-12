import { type Pokemon__Pokemon__output_type } from '../../Pokemon/Pokemon/output_type';

export type Query__HomePage__param = {
  /**
Returns a list of all the known Pokémon.

For every Pokémon all the data on each requested field is returned.

**_NOTE:_ To skip all CAP Pokémon, PokéStar Pokémon, Missingno, and 'M (00) provide an `offset` of 89**

You can provide `take` to limit the amount of Pokémon to return (default: 1), set the offset of where to start with `offset`, and reverse the entire array with `reverse`.

You can provide `takeFlavorTexts` to limit the amount of flavour texts to return, set the offset of where to start with `offsetFlavorTexts`, and reverse the entire array with `reverseFlavorTexts`.

While the API will currently not rate limit the usage of this query, it may do so in the future.

It is advisable to cache responses of this query.
  */
  getAllPokemon: ({
        /**
The key of the Pokémon as stored in the API
    */
key: string,
        /**
The form identifier of a Pokémon
    */
forme: (string | null),
    Pokemon: Pokemon__Pokemon__output_type,
  })[],
};
