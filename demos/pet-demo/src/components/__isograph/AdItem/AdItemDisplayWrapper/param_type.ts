import { type AdItem__AdItemDisplay__output_type } from '../../AdItem/AdItemDisplay/output_type';
import { type LoadableField, type ExtractParameters } from '@isograph/react';
import { type AdItem__AdItemDisplay__param } from '../../AdItem/AdItemDisplay/param_type';

export type AdItem__AdItemDisplayWrapper__param = {
  readonly data: {
    readonly AdItemDisplay: LoadableField<
      AdItem__AdItemDisplay__param,
      AdItem__AdItemDisplay__output_type
    >,
  },
  readonly parameters: Record<PropertyKey, never>,
};
