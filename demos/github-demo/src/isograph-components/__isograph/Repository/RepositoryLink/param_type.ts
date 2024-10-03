
import { type Variables } from '@isograph/react';

export type Repository__RepositoryLink__param = {
  readonly data: {
        /**
The Node ID of the Repository object
    */
readonly id: string,
        /**
The name of the repository.
    */
readonly name: string,
    /**
The User owner of the repository.
    */
    readonly owner: {
            /**
The username used to login.
      */
readonly login: string,
    },
  },
  readonly parameters: Variables,
};
