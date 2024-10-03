import { type Actor__UserLink__output_type } from '../../Actor/UserLink/output_type';
import { type PullRequest__PullRequestLink__output_type } from '../../PullRequest/PullRequestLink/output_type';
import { type PullRequest__createdAtFormatted__output_type } from '../../PullRequest/createdAtFormatted/output_type';

import { type Variables } from '@isograph/react';

export type PullRequestConnection__PullRequestTable__param = {
  readonly data: {
    /**
A list of edges.
    */
    readonly edges: (ReadonlyArray<({
      /**
The item at the end of the edge.
      */
      readonly node: ({
                /**
The Node ID of the PullRequest object
        */
readonly id: string,
        readonly PullRequestLink: PullRequest__PullRequestLink__output_type,
                /**
Identifies the pull request number.
        */
readonly number: number,
                /**
Identifies the pull request title.
        */
readonly title: string,
        /**
The actor who authored the comment.
        */
        readonly author: ({
          readonly UserLink: Actor__UserLink__output_type,
                    /**
The username of the actor.
          */
readonly login: string,
        } | null),
                /**
`true` if the pull request is closed
        */
readonly closed: boolean,
                /**
Returns a count of how many comments this pull request has received.
        */
readonly totalCommentsCount: (number | null),
        readonly createdAtFormatted: PullRequest__createdAtFormatted__output_type,
      } | null),
    } | null)> | null),
  },
  readonly parameters: Variables,
};
