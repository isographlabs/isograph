import { type BlogItem__BlogItemMoreDetail__output_type } from '../../BlogItem/BlogItemMoreDetail/output_type';
import { type Image__ImageDisplayWrapper__output_type } from '../../Image/ImageDisplayWrapper/output_type';

import { type LoadableField } from '@isograph/react';
import { type Variables } from '@isograph/react';

export type BlogItem__BlogItemDisplay__param = {
  readonly data: {
    readonly author: string,
    readonly title: string,
    readonly content: string,
    readonly BlogItemMoreDetail: LoadableField<void, BlogItem__BlogItemMoreDetail__output_type>,
    readonly image: ({
      readonly ImageDisplayWrapper: Image__ImageDisplayWrapper__output_type,
    } | null),
  },
  readonly parameters: Variables,
};
