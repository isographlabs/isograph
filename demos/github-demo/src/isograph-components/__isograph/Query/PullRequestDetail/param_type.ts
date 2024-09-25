import { type PullRequest__CommentList__output_type } from '../../PullRequest/CommentList/output_type';

import { type Variables } from '@isograph/react';

export type Query__PullRequestDetail__param = {
  data: {
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
        CommentList: PullRequest__CommentList__output_type,
      } | null),
    } | null),
  },
  parameters: Variables,
};
