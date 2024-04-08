import type  { Pet__set_best_friend__outputType}  from '../set_best_friend/reader';
import type  { Pet__set_pet_tagline__outputType}  from '../set_pet_tagline/reader';

export type Pet__PetUpdater__param = {
  set_best_friend: Pet__set_best_friend__outputType,
  potential_new_best_friends: ({
    id: string,
    name: string,
  })[],
  set_pet_tagline: Pet__set_pet_tagline__outputType,
  tagline: string,
};

            