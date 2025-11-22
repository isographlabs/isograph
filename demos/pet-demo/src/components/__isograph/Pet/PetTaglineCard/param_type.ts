import type { StartUpdate } from '@isograph/react';

export type Pet__PetTaglineCard__param = {
  readonly data: {
    /**
The pet's ID
    */
    readonly id: string,
    /**
If your pet was a superhero.
    */
    readonly tagline: string,
  },
  readonly parameters: Record<PropertyKey, never>,
  readonly startUpdate: StartUpdate<{
    /**
The pet's ID
    */
    readonly id: string,
    /**
If your pet was a superhero.
    */
    tagline: string,
  }>,
};
