import { type PullRequestConnection__PullRequestTable__output_type } from '../../PullRequestConnection/PullRequestTable/output_type';
import { type Repository__RepositoryLink__output_type } from '../../Repository/RepositoryLink/output_type';
import { type Starrable__IsStarred__output_type } from '../../Starrable/IsStarred/output_type';

export type Query__RepositoryDetail__param = {
  /**
Lookup a given repository by the owner and repository name.
  */
  repository: ({
    IsStarred: Starrable__IsStarred__output_type,
        /**
The repository's name with owner.
    */
nameWithOwner: string,
    /**
The repository parent, if this is a fork.
    */
    parent: ({
      RepositoryLink: Repository__RepositoryLink__output_type,
            /**
The repository's name with owner.
      */
nameWithOwner: string,
    } | null),
    /**
A list of pull requests that have been opened in the repository.
    */
    pullRequests: {
      PullRequestTable: PullRequestConnection__PullRequestTable__output_type,
    },
  } | null),
};
