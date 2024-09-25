import { type AdItem__AdItemDisplayWrapper__output_type } from '../../AdItem/AdItemDisplayWrapper/output_type';
import { type BlogItem__BlogItemDisplay__output_type } from '../../BlogItem/BlogItemDisplay/output_type';

import { type Variables } from '@isograph/react';

export type NewsfeedItem__NewsfeedAdOrBlog__param = {
  data: {
    adItem: ({
      AdItemDisplayWrapper: AdItem__AdItemDisplayWrapper__output_type,
    } | null),
    blogItem: ({
      BlogItemDisplay: BlogItem__BlogItemDisplay__output_type,
    } | null),
  },
  parameters: Variables,
};
