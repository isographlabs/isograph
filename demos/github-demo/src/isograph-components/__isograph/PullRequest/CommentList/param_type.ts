import { type IssueComment__formattedCommentCreationDate__output_type } from '../../IssueComment/formattedCommentCreationDate/output_type';

export type PullRequest__CommentList__param = {
  /**
A list of comments associated with the pull request.
  */
  comments: {
    /**
A list of edges.
    */
    edges: (({
      /**
The item at the end of the edge.
      */
      node: ({
                /**
The Node ID of the IssueComment object
        */
id: string,
                /**
The body rendered to text.
        */
bodyText: string,
        formattedCommentCreationDate: IssueComment__formattedCommentCreationDate__output_type,
        /**
The actor who authored the comment.
        */
        author: ({
                    /**
The username of the actor.
          */
login: string,
        } | null),
      } | null),
    } | null))[],
  },
};
