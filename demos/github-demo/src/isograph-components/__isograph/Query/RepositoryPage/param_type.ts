import { type Query__Header__output_type } from '../../Query/Header/output_type';
import { type Query__RepositoryDetail__output_type } from '../../Query/RepositoryDetail/output_type';

import { type Variables } from '@isograph/react';

export type Query__RepositoryPage__param = {
  data: {
    Header: Query__Header__output_type,
    RepositoryDetail: Query__RepositoryDetail__output_type,
  },
  parameters: Variables,
};
