import {PullRequestConnection__PullRequestTable__outputType} from '../../PullRequestConnection/PullRequestTable/output_type';
import {Repository__RepositoryLink__outputType} from '../../Repository/RepositoryLink/output_type';
import {Starrable__IsStarred__outputType} from '../../Starrable/IsStarred/output_type';

export type Query__RepositoryDetail__param = {
  /**
Lookup a given repository by the owner and repository name.
  */
  repository: ({
    IsStarred: Starrable__IsStarred__outputType,
        /**
The repository's name with owner.
    */
nameWithOwner: string,
    /**
The repository parent, if this is a fork.
    */
    parent: ({
      RepositoryLink: Repository__RepositoryLink__outputType,
            /**
The repository's name with owner.
      */
nameWithOwner: string,
    } | null),
    /**
A list of pull requests that have been opened in the repository.
    */
    pullRequests: {
      PullRequestTable: PullRequestConnection__PullRequestTable__outputType,
    },
  } | null),
};
