import { type Query__Header__output_type } from '../../Query/Header/output_type';
import { type Query__HomePageList__output_type } from '../../Query/HomePageList/output_type';

import { type Variables } from '@isograph/react';

export type Query__HomePage__param = {
  data: {
    Header: Query__Header__output_type,
    HomePageList: Query__HomePageList__output_type,
  },
  parameters: Variables,
};
