import { type User__Avatar__output_type } from '../../User/Avatar/output_type';

import { type Variables } from '@isograph/react';

export type Query__Header__param = {
  readonly data: {
    /**
The currently authenticated user.
    */
    readonly viewer: {
            /**
The user's public profile name.
      */
readonly name: (string | null),
      readonly Avatar: User__Avatar__output_type,
    },
  },
  readonly parameters: Variables,
};
