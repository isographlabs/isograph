import {Pet__PetUpdater__outputType} from '../PetUpdater/output_type';

export type Pet__PetBestFriendCard__param = {
  id: string,
  PetUpdater: Pet__PetUpdater__outputType,
  best_friend_relationship: ({
    picture_together: (string | null),
    best_friend: {
      id: string,
      name: string,
      picture: string,
    },
  } | null),
};
