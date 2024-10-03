
import { type Variables } from '@isograph/react';

export type Query__meNameSuccessor__param = {
  readonly data: {
    readonly me: {
      readonly name: string,
      readonly successor: ({
        readonly successor: ({
          readonly name: string,
        } | null),
      } | null),
    },
  },
  readonly parameters: Variables,
};
