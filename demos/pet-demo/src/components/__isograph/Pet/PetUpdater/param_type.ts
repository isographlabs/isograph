import { type Pet____refetch__output_type } from '../../Pet/__refetch/output_type';
import { type Pet__set_best_friend__output_type } from '../../Pet/set_best_friend/output_type';
import { type Pet__set_pet_tagline__output_type } from '../../Pet/set_pet_tagline/output_type';

export type Pet__PetUpdater__param = {
  set_best_friend: Pet__set_best_friend__output_type,
  potential_new_best_friends: ({
    id: string,
    name: string,
  })[],
  set_pet_tagline: Pet__set_pet_tagline__output_type,
  tagline: string,
  /**
A refetch field for the Pet type.
  */
  __refetch: Pet____refetch__output_type,
};
