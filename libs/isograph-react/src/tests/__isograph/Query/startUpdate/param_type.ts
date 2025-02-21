import type { StartUpdate } from '@isograph/react';
import type { Query__startUpdate__parameters } from './parameters_type';

export type Query__startUpdate__param = {
  readonly data: {
    readonly node: ({
      /**
A client pointer for the Economist type.
      */
      readonly asEconomist: ({
        readonly name: string,
      } | null),
    } | null),
  },
  readonly parameters: Query__startUpdate__parameters,
  readonly startUpdate: StartUpdate<{
    readonly node: ({
      /**
A client pointer for the Economist type.
      */
      readonly asEconomist: ({
        name: string,
      } | null),
    } | null),
  }>,
};
