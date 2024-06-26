import { type Pet__PetUpdater__output_type } from '../../Pet/PetUpdater/output_type';

export type Pet__PetBestFriendCard__param = {
  id: string,
  /**
# Pet.PetUpdater
A component to test behavior with respect to mutations.
You can update the best friend and the tagline.
  */
  PetUpdater: Pet__PetUpdater__output_type,
  best_friend_relationship: ({
    picture_together: (string | null),
    best_friend: {
      id: string,
      name: string,
      picture: string,
    },
  } | null),
};
