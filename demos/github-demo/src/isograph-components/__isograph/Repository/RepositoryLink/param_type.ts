
import { type Variables } from '@isograph/react';

export type Repository__RepositoryLink__param = {
  data: {
        /**
The Node ID of the Repository object
    */
id: string,
        /**
The name of the repository.
    */
name: string,
    /**
The User owner of the repository.
    */
    owner: {
            /**
The username used to login.
      */
login: string,
    },
  },
  parameters: Variables,
};
