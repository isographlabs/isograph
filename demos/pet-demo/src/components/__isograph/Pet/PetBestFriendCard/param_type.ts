import { type Pet__Avatar__output_type } from '../../Pet/Avatar/output_type';
import { type Pet__PetUpdater__output_type } from '../../Pet/PetUpdater/output_type';
import { type Pet__fullName__output_type } from '../../Pet/fullName/output_type';

export type Pet__PetBestFriendCard__param = {
  readonly data: {
    /**
The pet's ID
    */
    readonly id: string,
    /**
Pet.PetUpdater
A component to test behavior with respect to mutations.
You can update the best friend and the tagline.
    */
    readonly PetUpdater: Pet__PetUpdater__output_type,
    /**
Pets have a very complex social life. Find out more!
    */
    readonly best_friend_relationship: ({
      readonly picture_together: (unknown | null),
      readonly best_friend: {
        /**
The pet's ID
        */
        readonly id: string,
        readonly fullName: Pet__fullName__output_type,
        /**
A picture of a pet, framed.
        */
        readonly Avatar: Pet__Avatar__output_type,
      },
    } | null),
  },
  readonly parameters: Record<PropertyKey, never>,
};
