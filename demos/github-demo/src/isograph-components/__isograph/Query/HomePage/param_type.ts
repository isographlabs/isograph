import { type Query__Header__output_type } from '../../Query/Header/output_type';
import { type Query__HomePageList__output_type } from '../../Query/HomePageList/output_type';

import { type Variables } from '@isograph/react';

export type Query__HomePage__param = {
  readonly data: {
    readonly Header: Query__Header__output_type,
    readonly HomePageList: Query__HomePageList__output_type,
  },
  readonly parameters: Variables,
};
