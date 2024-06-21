import { type Actor__UserLink__output_type } from '../../Actor/UserLink/output_type';
import { type PullRequest__PullRequestLink__output_type } from '../../PullRequest/PullRequestLink/output_type';
import { type PullRequest__createdAtFormatted__output_type } from '../../PullRequest/createdAtFormatted/output_type';

export type PullRequestConnection__PullRequestTable__param = {
  /**
A list of edges.
  */
  edges: (({
    /**
The item at the end of the edge.
    */
    node: ({
            /**
The Node ID of the PullRequest object
      */
id: string,
      PullRequestLink: PullRequest__PullRequestLink__output_type,
            /**
Identifies the pull request number.
      */
number: number,
            /**
Identifies the pull request title.
      */
title: string,
      /**
The actor who authored the comment.
      */
      author: ({
        UserLink: Actor__UserLink__output_type,
                /**
The username of the actor.
        */
login: string,
      } | null),
            /**
`true` if the pull request is closed
      */
closed: boolean,
            /**
Returns a count of how many comments this pull request has received.
      */
totalCommentsCount: (number | null),
      createdAtFormatted: PullRequest__createdAtFormatted__output_type,
    } | null),
  } | null))[],
};
