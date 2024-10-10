import { type Image__ImageDisplay__output_type } from '../../Image/ImageDisplay/output_type';
import { type LoadableField } from '@isograph/react';
import { type Image__ImageDisplay__param } from '../../Image/ImageDisplay/param_type';

export type Image__ImageDisplayWrapper__param = {
  readonly data: {
    readonly ImageDisplay: LoadableField<
      Image__ImageDisplay__param,
      Image__ImageDisplay__output_type
    >,
  },
  readonly parameters: Record<string, never>,
};
