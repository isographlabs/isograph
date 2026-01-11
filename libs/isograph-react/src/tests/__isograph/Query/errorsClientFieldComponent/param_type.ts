import { type Economist__errorsClientFieldComponentField__output_type } from '../../Economist/errorsClientFieldComponentField/output_type';
import type { Query__errorsClientFieldComponent__parameters } from './parameters_type';

export type Query__errorsClientFieldComponent__param = {
  readonly data: {
    readonly node: ({
      /**
A client pointer for the Economist type.
      */
      readonly asEconomist: ({
        readonly errorsClientFieldComponentField: Economist__errorsClientFieldComponentField__output_type,
      } | null),
    } | null),
  },
  readonly parameters: Query__errorsClientFieldComponent__parameters,
};
