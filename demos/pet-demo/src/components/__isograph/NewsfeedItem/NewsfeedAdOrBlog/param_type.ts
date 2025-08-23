import { type AdItem__AdItemDisplay__output_type } from '../../AdItem/AdItemDisplay/output_type';
import { type BlogItem__BlogItemDisplay__output_type } from '../../BlogItem/BlogItemDisplay/output_type';
import { type LoadableField, type ExtractParameters } from '@isograph/react';
import { type AdItem__AdItemDisplay__param } from '../../AdItem/AdItemDisplay/param_type';

export type NewsfeedItem__NewsfeedAdOrBlog__param = {
  readonly data: {
    /**
A client pointer for the AdItem type.
    */
    readonly asAdItem: ({
      readonly AdItemDisplay: LoadableField<
        AdItem__AdItemDisplay__param,
        AdItem__AdItemDisplay__output_type
      >,
    } | null),
    /**
A client pointer for the BlogItem type.
    */
    readonly asBlogItem: ({
      readonly BlogItemDisplay: BlogItem__BlogItemDisplay__output_type,
    } | null),
  },
  readonly parameters: Record<PropertyKey, never>,
};
