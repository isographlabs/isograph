import { type NewsfeedItem__NewsfeedAdOrBlog__output_type } from '../../NewsfeedItem/NewsfeedAdOrBlog/output_type';

import { type Variables } from '@isograph/react';

export type Viewer__NewsfeedPaginationComponent__param = {
  readonly data: {
    readonly newsfeed: ReadonlyArray<{
      readonly NewsfeedAdOrBlog: NewsfeedItem__NewsfeedAdOrBlog__output_type,
    }>,
  },
  readonly parameters: Variables,
};
