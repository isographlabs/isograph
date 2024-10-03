import { type Query__Header__output_type } from '../../Query/Header/output_type';
import { type Query__UserDetail__output_type } from '../../Query/UserDetail/output_type';

import { type Variables } from '@isograph/react';

export type Query__UserPage__param = {
  readonly data: {
    readonly Header: Query__Header__output_type,
    readonly UserDetail: Query__UserDetail__output_type,
  },
  readonly parameters: Variables,
};
