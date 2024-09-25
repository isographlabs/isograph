
import { type Variables } from '@isograph/react';

export type Query__meNameSuccessor__param = {
  data: {
    me: {
      name: string,
      successor: ({
        successor: ({
          name: string,
        } | null),
      } | null),
    },
  },
  parameters: Variables,
};
