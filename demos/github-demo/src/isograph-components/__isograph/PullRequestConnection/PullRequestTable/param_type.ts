import {Actor__UserLink__outputType} from '../../Actor/UserLink/output_type';
import {PullRequest__PullRequestLink__outputType} from '../../PullRequest/PullRequestLink/output_type';
import {PullRequest__createdAtFormatted__outputType} from '../../PullRequest/createdAtFormatted/output_type';

export type PullRequestConnection__PullRequestTable__param = {
  edges: (({
    node: ({
      id: string,
      PullRequestLink: PullRequest__PullRequestLink__outputType,
      number: number,
      title: string,
      author: ({
        UserLink: Actor__UserLink__outputType,
        login: string,
      } | null),
      closed: boolean,
      totalCommentsCount: (number | null),
      createdAtFormatted: PullRequest__createdAtFormatted__outputType,
    } | null),
  } | null))[],
};
