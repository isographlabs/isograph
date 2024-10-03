import { type PullRequestConnection__PullRequestTable__output_type } from '../../PullRequestConnection/PullRequestTable/output_type';
import { type Repository__RepositoryLink__output_type } from '../../Repository/RepositoryLink/output_type';
import { type Starrable__IsStarred__output_type } from '../../Starrable/IsStarred/output_type';

import { type Variables } from '@isograph/react';

export type Query__RepositoryDetail__param = {
  readonly data: {
    /**
Lookup a given repository by the owner and repository name.
    */
    readonly repository: ({
      readonly IsStarred: Starrable__IsStarred__output_type,
            /**
The repository's name with owner.
      */
readonly nameWithOwner: string,
      /**
The repository parent, if this is a fork.
      */
      readonly parent: ({
        readonly RepositoryLink: Repository__RepositoryLink__output_type,
                /**
The repository's name with owner.
        */
readonly nameWithOwner: string,
      } | null),
      /**
A list of pull requests that have been opened in the repository.
      */
      readonly pullRequests: {
        readonly PullRequestTable: PullRequestConnection__PullRequestTable__output_type,
      },
    } | null),
  },
  readonly parameters: Variables,
};
