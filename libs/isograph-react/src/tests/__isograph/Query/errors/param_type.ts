import type { Query__errors__parameters } from './parameters_type';

export type Query__errors__param = {
  readonly data: {
    readonly node: ({
      /**
A client pointer for the Economist type.
      */
      readonly asEconomist: ({
        readonly id: string,
        readonly name: string,
      } | null),
    } | null),
  },
  readonly parameters: Query__errors__parameters,
};
