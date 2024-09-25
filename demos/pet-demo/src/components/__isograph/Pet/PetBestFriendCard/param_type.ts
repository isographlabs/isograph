import { type Pet__PetUpdater__output_type } from '../../Pet/PetUpdater/output_type';

import { type Variables } from '@isograph/react';

export type Pet__PetBestFriendCard__param = {
  data: {
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
  },
  parameters: Variables,
};
