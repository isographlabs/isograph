import { type Query__Header__output_type } from '../../Query/Header/output_type';
import { type Query__PullRequestDetail__output_type } from '../../Query/PullRequestDetail/output_type';

import { type Variables } from '@isograph/react';

export type Query__PullRequest__param = {
  data: {
    Header: Query__Header__output_type,
    PullRequestDetail: Query__PullRequestDetail__output_type,
  },
  parameters: Variables,
};
