import React from 'react';
import { iso } from '@iso';

export const NewsfeedPaginationComponent = iso(`
  field Viewer.NewsfeedPaginationComponent($skip: Int!, $limit: Int!) {
    newsfeed(skip: $skip, limit: $limit, additionalSkip: 5) {
      NewsfeedAdOrBlog
    }
  }
`)(({ data }) => {
  return data.newsfeed;
});
