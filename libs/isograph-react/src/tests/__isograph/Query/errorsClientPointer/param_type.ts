import { type LoadableField, type ExtractParameters } from '@isograph/react';
import { type Economist__errorsClientPointerField__param } from '../../Economist/errorsClientPointerField/param_type';
import type { Query__errorsClientPointer__parameters } from './parameters_type';

export type Query__errorsClientPointer__param = {
  readonly data: {
    readonly node: ({
      /**
A client pointer for the Economist type.
      */
      readonly asEconomist: ({
        readonly errorsClientPointerField: (LoadableField<Economist__errorsClientPointerField__param, {
          readonly id: string,
        }> | null),
      } | null),
    } | null),
  },
  readonly parameters: Query__errorsClientPointer__parameters,
};
