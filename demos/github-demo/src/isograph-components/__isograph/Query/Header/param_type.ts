import { type User__Avatar__output_type } from '../../User/Avatar/output_type';

import { type Variables } from '@isograph/react';

export type Query__Header__param = {
  data: {
    /**
The currently authenticated user.
    */
    viewer: {
            /**
The user's public profile name.
      */
name: (string | null),
      Avatar: User__Avatar__output_type,
    },
  },
  parameters: Variables,
};
