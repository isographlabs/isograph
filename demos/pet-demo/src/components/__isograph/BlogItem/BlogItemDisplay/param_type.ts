import { type BlogItem__BlogItemMoreDetail__output_type } from '../../BlogItem/BlogItemMoreDetail/output_type';
import { type Image__ImageDisplayWrapper__output_type } from '../../Image/ImageDisplayWrapper/output_type';

import { type LoadableField } from '@isograph/react';
export type BlogItem__BlogItemDisplay__param = {
  author: string,
  title: string,
  content: string,
  BlogItemMoreDetail: LoadableField<void, BlogItem__BlogItemMoreDetail__output_type>,
  image: ({
    ImageDisplayWrapper: Image__ImageDisplayWrapper__output_type,
  } | null),
};
