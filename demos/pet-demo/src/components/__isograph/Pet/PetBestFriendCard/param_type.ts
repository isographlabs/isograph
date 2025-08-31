import { type Pet__Avatar__output_type } from '../../Pet/Avatar/output_type';
import { type Pet__PetUpdater__output_type } from '../../Pet/PetUpdater/output_type';

export type Pet__PetBestFriendCard__param = {
  readonly data: {
    readonly id: string,
    /**
Pet.PetUpdater
A component to test behavior with respect to mutations.
You can update the best friend and the tagline.
    */
    readonly PetUpdater: Pet__PetUpdater__output_type,
    readonly best_friend_relationship: ({
      readonly picture_together: (string | null),
      readonly best_friend: {
        readonly id: string,
        readonly name: string,
        readonly Avatar: Pet__Avatar__output_type,
      },
    } | null),
  },
  readonly parameters: Record<PropertyKey, never>,
};
