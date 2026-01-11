import { type Economist__errorsClientFieldField__output_type } from '../../Economist/errorsClientFieldField/output_type';
import type { Query__errorsClientField__parameters } from './parameters_type';

export type Query__errorsClientField__param = {
  readonly data: {
    readonly node: ({
      /**
A client pointer for the Economist type.
      */
      readonly asEconomist: ({
        readonly errorsClientFieldField: Economist__errorsClientFieldField__output_type,
      } | null),
    } | null),
  },
  readonly parameters: Query__errorsClientField__parameters,
};
