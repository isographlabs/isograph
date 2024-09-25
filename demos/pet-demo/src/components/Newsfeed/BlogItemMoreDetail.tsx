import React from 'react';
import { iso } from '@iso';

export const BlogItemMoreDetail = iso(`
  field BlogItem.BlogItemMoreDetail @component {
    moreContent
  }
`)(({ data: blogItem }) => {
  return blogItem.moreContent
    .split('\n')
    .map((paragraph, index) => <p key={index}>{paragraph}</p>);
});
