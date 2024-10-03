import { type NewsfeedItem__NewsfeedAdOrBlog__output_type } from '../../NewsfeedItem/NewsfeedAdOrBlog/output_type';
import { type Viewer__NewsfeedPaginationComponent__output_type } from '../../Viewer/NewsfeedPaginationComponent/output_type';

import { type LoadableField } from '@isograph/react';
import { type Variables } from '@isograph/react';

export type Query__Newsfeed__param = {
  readonly data: {
    readonly viewer: {
      readonly newsfeed: ReadonlyArray<{
        readonly NewsfeedAdOrBlog: NewsfeedItem__NewsfeedAdOrBlog__output_type,
      }>,
      readonly NewsfeedPaginationComponent: LoadableField<{readonly skip: number, readonly limit: number}, Viewer__NewsfeedPaginationComponent__output_type>,
    },
  },
  readonly parameters: Variables,
};
