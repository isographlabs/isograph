import {PullRequestConnection__PullRequestTable__outputType} from '../../PullRequestConnection/PullRequestTable/output_type';
import {Repository__RepositoryLink__outputType} from '../../Repository/RepositoryLink/output_type';
import {Starrable__IsStarred__outputType} from '../../Starrable/IsStarred/output_type';

export type Query__RepositoryDetail__param = {
  repository: ({
    IsStarred: Starrable__IsStarred__outputType,
    nameWithOwner: string,
    parent: ({
      RepositoryLink: Repository__RepositoryLink__outputType,
      nameWithOwner: string,
    } | null),
    pullRequests: {
      PullRequestTable: PullRequestConnection__PullRequestTable__outputType,
    },
  } | null),
};
