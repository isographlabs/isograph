import {PullRequest__CommentList__outputType} from '../../PullRequest/CommentList/output_type';

export type Query__PullRequestDetail__param = {
  repository: ({
    pullRequest: ({
      title: string,
      bodyHTML: string,
      CommentList: PullRequest__CommentList__outputType,
    } | null),
  } | null),
};
