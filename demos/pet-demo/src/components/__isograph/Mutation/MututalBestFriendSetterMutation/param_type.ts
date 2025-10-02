import { type Mutation__MutualBestFriendSetterOtherSide__output_type } from '../../Mutation/MutualBestFriendSetterOtherSide/output_type';
import { type LoadableField, type ExtractParameters } from '@isograph/react';
import { type Mutation__MutualBestFriendSetterOtherSide__param } from '../../Mutation/MutualBestFriendSetterOtherSide/param_type';
import type { Mutation__MututalBestFriendSetterMutation__parameters } from './parameters_type';

export type Mutation__MututalBestFriendSetterMutation__param = {
  readonly data: {
    readonly set_pet_best_friend: {
      readonly pet: {
        readonly id: string,
        readonly best_friend_relationship: ({
          readonly best_friend: {
            readonly id: string,
          },
        } | null),
      },
    },
    readonly MutualBestFriendSetterOtherSide: LoadableField<
      Mutation__MutualBestFriendSetterOtherSide__param,
      Mutation__MutualBestFriendSetterOtherSide__output_type
    >,
  },
  readonly parameters: Mutation__MututalBestFriendSetterMutation__parameters,
};
