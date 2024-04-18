import {PullRequest__CommentList__outputType} from '../../PullRequest/CommentList/output_type';

export type Query__PullRequestDetail__param = {
  /**
Lookup a given repository by the owner and repository name.
  */
  repository: ({
    /**
Returns a single pull request from the current repository by number.
    */
    pullRequest: ({
            /**
Identifies the pull request title.
      */
title: string,
            /**
The body rendered to HTML.
      */
bodyHTML: string,
      CommentList: PullRequest__CommentList__outputType,
    } | null),
  } | null),
};
