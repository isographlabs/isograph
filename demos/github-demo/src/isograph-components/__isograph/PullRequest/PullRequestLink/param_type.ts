
import { type Variables } from '@isograph/react';

export type PullRequest__PullRequestLink__param = {
  data: {
        /**
Identifies the pull request number.
    */
number: number,
    /**
The repository associated with this node.
    */
    repository: {
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
  },
  parameters: Variables,
};
