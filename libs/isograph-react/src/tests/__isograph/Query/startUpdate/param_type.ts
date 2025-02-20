import { type Economist__omitted__output_type } from '../../Economist/omitted/output_type';
import type { StartUpdate } from '@isograph/react';
import type { Query__startUpdate__parameters } from './parameters_type';

export type Query__startUpdate__param = {
  readonly data: {
    readonly node: ({
      /**
A client pointer for the Economist type.
      */
      readonly asEconomist: ({
        readonly omitted: Economist__omitted__output_type,
        readonly id: string,
        readonly name: string,
        readonly successor: ({
          readonly id: string,
          readonly name: string,
        } | null),
      } | null),
    } | null),
  },
  readonly parameters: Query__startUpdate__parameters,
  readonly startUpdate: StartUpdate<{
    readonly node: ({
      /**
A client pointer for the Economist type.
      */
      get asEconomist(): ({
        readonly omitted: Economist__omitted__output_type,
        readonly id: string,
        name: string,
        readonly successor: ({
          readonly id: string,
          readonly name: string,
        } | null),
      } | null),
      set asEconomist(value: ({
        readonly id: string,
        readonly name: string,
        readonly successor: ({
          readonly id: string,
          readonly name: string,
        } | null),
      } | null)),
    } | null),
  }>,
};
