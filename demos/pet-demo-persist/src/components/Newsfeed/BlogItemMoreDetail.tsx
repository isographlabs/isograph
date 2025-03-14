import { iso } from '@iso';
import React from 'react';

export const BlogItemMoreDetail = iso(`
  field BlogItem.BlogItemMoreDetail @component {
    moreContent
  }
`)(({ data: blogItem }) => {
  return blogItem.moreContent
    .split('\n')
    .map((paragraph, index) => <p key={index}>{paragraph}</p>);
});
