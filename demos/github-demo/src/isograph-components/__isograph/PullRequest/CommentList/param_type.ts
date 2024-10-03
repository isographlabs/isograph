import { type IssueComment__formattedCommentCreationDate__output_type } from '../../IssueComment/formattedCommentCreationDate/output_type';

import { type Variables } from '@isograph/react';

export type PullRequest__CommentList__param = {
  readonly data: {
    /**
A list of comments associated with the pull request.
    */
    readonly comments: {
      /**
A list of edges.
      */
      readonly edges: (ReadonlyArray<({
        /**
The item at the end of the edge.
        */
        readonly node: ({
                    /**
The Node ID of the IssueComment object
          */
readonly id: string,
                    /**
The body rendered to text.
          */
readonly bodyText: string,
          readonly formattedCommentCreationDate: IssueComment__formattedCommentCreationDate__output_type,
          /**
The actor who authored the comment.
          */
          readonly author: ({
                        /**
The username of the actor.
            */
readonly login: string,
          } | null),
        } | null),
      } | null)> | null),
    },
  },
  readonly parameters: Variables,
};
