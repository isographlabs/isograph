import { type AdItem__AdItemDisplayWrapper__output_type } from '../../AdItem/AdItemDisplayWrapper/output_type';
import { type BlogItem__BlogItemDisplay__output_type } from '../../BlogItem/BlogItemDisplay/output_type';

export type NewsfeedItem__NewsfeedAdOrBlog__param = {
  readonly data: {
    /**
A client pointer for the AdItem type.
    */
    readonly asAdItem: ({
      readonly AdItemDisplayWrapper: AdItem__AdItemDisplayWrapper__output_type,
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
