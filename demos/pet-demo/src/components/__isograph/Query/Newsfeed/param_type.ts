import { type NewsfeedItem__NewsfeedAdOrBlog__output_type } from '../../NewsfeedItem/NewsfeedAdOrBlog/output_type';
import { type Viewer__NewsfeedPaginationComponent__output_type } from '../../Viewer/NewsfeedPaginationComponent/output_type';
import { type LoadableField, type ExtractParameters } from '@isograph/react';
import { type Viewer__NewsfeedPaginationComponent__param } from '../../Viewer/NewsfeedPaginationComponent/param_type';

export type Query__Newsfeed__param = {
  readonly data: {
    readonly viewer: {
      readonly newsfeed: ReadonlyArray<{
        readonly NewsfeedAdOrBlog: NewsfeedItem__NewsfeedAdOrBlog__output_type,
      }>,
      readonly NewsfeedPaginationComponent: LoadableField<
        Viewer__NewsfeedPaginationComponent__param,
        Viewer__NewsfeedPaginationComponent__output_type
      >,
    },
  },
  readonly parameters: Record<string, never>,
};
