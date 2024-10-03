import { type User__RepositoryList__output_type } from '../../User/RepositoryList/output_type';
import { type User____refetch__output_type } from '../../User/__refetch/output_type';

import { type Variables } from '@isograph/react';

export type Query__HomePageList__param = {
  readonly data: {
    /**
The currently authenticated user.
    */
    readonly viewer: {
            /**
The username used to login.
      */
readonly login: string,
            /**
The user's public profile name.
      */
readonly name: (string | null),
      readonly RepositoryList: User__RepositoryList__output_type,
      /**
A refetch field for the User type.
      */
      readonly __refetch: User____refetch__output_type,
    },
  },
  readonly parameters: Variables,
};
