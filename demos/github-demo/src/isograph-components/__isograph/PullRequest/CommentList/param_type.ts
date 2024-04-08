import {IssueComment__formattedCommentCreationDate__outputType} from '../../IssueComment/formattedCommentCreationDate/output_type';

export type PullRequest__CommentList__param = {
  comments: {
    edges: (({
      node: ({
        id: string,
        bodyText: string,
        formattedCommentCreationDate: IssueComment__formattedCommentCreationDate__outputType,
        author: ({
          login: string,
        } | null),
      } | null),
    } | null))[],
  },
};
