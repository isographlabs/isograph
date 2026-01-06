import { type PullRequest__CommentList__output_type } from '../../PullRequest/CommentList/output_type';
import type { Query__PullRequestDetail__parameters } from './parameters_type';

export type Query__PullRequestDetail__param = {
  readonly data: {
    /**
Lookup a given repository by the owner and repository name.
    */
    readonly repository: ({
      /**
Returns a single pull request from the current repository by number.
      */
      readonly pullRequest: ({
        /**
Identifies the pull request title.
        */
        readonly title: string,
        /**
The body rendered to HTML.
        */
        readonly bodyHTML: unknown,
        readonly CommentList: PullRequest__CommentList__output_type,
      } | null),
    } | null),
  },
  readonly parameters: Query__PullRequestDetail__parameters,
};
