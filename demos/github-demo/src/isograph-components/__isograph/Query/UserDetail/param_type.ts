import { type User__RepositoryList__output_type } from '../../User/RepositoryList/output_type';

import { type Variables } from '@isograph/react';

export type Query__UserDetail__param = {
  data: {
    /**
Lookup a user by login.
    */
    user: ({
            /**
The user's public profile name.
      */
name: (string | null),
      RepositoryList: User__RepositoryList__output_type,
    } | null),
  },
  parameters: Variables,
};
