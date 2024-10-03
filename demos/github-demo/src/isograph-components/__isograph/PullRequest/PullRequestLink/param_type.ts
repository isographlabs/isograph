
import { type Variables } from '@isograph/react';

export type PullRequest__PullRequestLink__param = {
  readonly data: {
        /**
Identifies the pull request number.
    */
readonly number: number,
    /**
The repository associated with this node.
    */
    readonly repository: {
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
  },
  readonly parameters: Variables,
};
