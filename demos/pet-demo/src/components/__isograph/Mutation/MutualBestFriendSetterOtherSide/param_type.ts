import { type Pet__fullName__output_type } from '../../Pet/fullName/output_type';
import type { Mutation__MutualBestFriendSetterOtherSide__parameters } from './parameters_type';

export type Mutation__MutualBestFriendSetterOtherSide__param = {
  readonly data: {
    readonly set_pet_best_friend: {
      readonly pet: {
        /**
The pet's ID
        */
        readonly id: string,
        readonly fullName: Pet__fullName__output_type,
        /**
Pets have a very complex social life. Find out more!
        */
        readonly best_friend_relationship: ({
          readonly best_friend: {
            /**
The pet's ID
            */
            readonly id: string,
            readonly fullName: Pet__fullName__output_type,
          },
        } | null),
      },
    },
  },
  readonly parameters: Mutation__MutualBestFriendSetterOtherSide__parameters,
};
