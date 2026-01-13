import type { Query__nicknameErrors__parameters } from './parameters_type';

export type Query__nicknameErrors__param = {
  readonly data: {
    readonly node: ({
      /**
A client pointer for the Economist type.
      */
      readonly asEconomist: ({
        readonly id: string,
        readonly nickname: (string | null),
      } | null),
    } | null),
  },
  readonly parameters: Query__nicknameErrors__parameters,
};
