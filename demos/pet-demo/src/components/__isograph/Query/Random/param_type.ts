import { type LoadableField, type ExtractParameters } from '@isograph/react';
import { type Query__firstNode__param } from '../../Query/firstNode/param_type';

export type Query__Random__param = {
  readonly data: {
    readonly firstNode: LoadableField<Query__firstNode__param, ({
      readonly nickname: (string | null),
    } | null)>,
  },
  readonly parameters: Record<PropertyKey, never>,
};
