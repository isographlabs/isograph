import type { Mutation__MutualBestFriendSetterOtherSide__parameters } from './parameters_type';

export type Mutation__MutualBestFriendSetterOtherSide__param = {
  readonly data: {
    readonly set_pet_best_friend: {
      readonly pet: {
        readonly id: string,
        readonly name: string,
        readonly best_friend_relationship: ({
          readonly best_friend: {
            readonly id: string,
            readonly name: string,
          },
        } | null),
      },
    },
  },
  readonly parameters: Mutation__MutualBestFriendSetterOtherSide__parameters,
};
