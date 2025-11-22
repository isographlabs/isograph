import { type Pet____refetch__output_type } from '../../Pet/__refetch/output_type';
import { type Pet__fullName__output_type } from '../../Pet/fullName/output_type';
import { type Pet__set_best_friend__output_type } from '../../Pet/set_best_friend/output_type';
import { type Pet__set_pet_tagline__output_type } from '../../Pet/set_pet_tagline/output_type';
import type { StartUpdate } from '@isograph/react';

export type Pet__PetUpdater__param = {
  readonly data: {
    readonly set_best_friend: Pet__set_best_friend__output_type,
    /**
Hold your enemies close...
    */
    readonly potential_new_best_friends: ReadonlyArray<{
      /**
The pet's ID
      */
      readonly id: string,
      readonly fullName: Pet__fullName__output_type,
    }>,
    readonly set_pet_tagline: Pet__set_pet_tagline__output_type,
    /**
If your pet was a superhero.
    */
    readonly tagline: string,
    /**
A refetch field for the Pet type.
    */
    readonly __refetch: Pet____refetch__output_type,
  },
  readonly parameters: Record<PropertyKey, never>,
  readonly startUpdate: StartUpdate<{
    readonly set_best_friend: Pet__set_best_friend__output_type,
    /**
Hold your enemies close...
    */
    readonly potential_new_best_friends: ReadonlyArray<{
      /**
The pet's ID
      */
      readonly id: string,
      readonly fullName: Pet__fullName__output_type,
    }>,
    readonly set_pet_tagline: Pet__set_pet_tagline__output_type,
    /**
If your pet was a superhero.
    */
    tagline: string,
    /**
A refetch field for the Pet type.
    */
    readonly __refetch: Pet____refetch__output_type,
  }>,
};
