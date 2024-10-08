import { type Query__Header__output_type } from '../../Query/Header/output_type';
import { type Query__PullRequestDetail__output_type } from '../../Query/PullRequestDetail/output_type';
import type { Query__PullRequest__parameters } from './parameters_type';

export type Query__PullRequest__param = {
  readonly data: {
    readonly Header: Query__Header__output_type,
    readonly PullRequestDetail: Query__PullRequestDetail__output_type,
  },
  readonly parameters: Query__PullRequest__parameters,
};
